//! Quern VM GUI 引擎模块
//! 基于 egui + eframe 实现 QB 脚本内的 GUI 窗口渲染
//!
//! QB 语法示例:
//! ```
//! gui crt wdw{
//!   tit "我的窗口"
//!   lbl "你好世界"
//!   btn "点击我" @btn_state@
//!   txt "输入文字" @input_text@
//! }
//! out @btn_state@
//! out @input_text@
//! ```

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use egui;
use eframe;

use crate::instructions::GuiWidget;

// ============================================================
// GUI 运行时状态（通过 Arc<Mutex<>> 在 egui App 和 QVM 之间共享）
// ============================================================

/// 按钮点击状态
#[derive(Debug, Clone, Default)]
struct ButtonState {
    clicked: bool,
}

/// 文本输入状态
#[derive(Debug, Clone, Default)]
struct TextInputState {
    value: String,
}

/// GUI 应用共享状态
pub struct GuiAppState {
    /// 窗口标题
    pub title: String,
    /// 控件定义
    pub widgets: Vec<GuiWidget>,
    /// 按钮点击状态: var_name -> clicked
    pub button_states: HashMap<String, ButtonState>,
    /// 文本输入状态: var_name -> value
    pub text_states: HashMap<String, TextInputState>,
    /// 是否已关闭窗口
    pub closed: bool,
}

impl GuiAppState {
    pub fn new(title: String, widgets: Vec<GuiWidget>) -> Self {
        let mut button_states = HashMap::new();
        let mut text_states = HashMap::new();

        for widget in &widgets {
            match widget {
                GuiWidget::Button { var_name, .. } => {
                    button_states.insert(var_name.clone(), ButtonState::default());
                }
                GuiWidget::TextInput { var_name, .. } => {
                    text_states.insert(var_name.clone(), TextInputState::default());
                }
                _ => {}
            }
        }

        GuiAppState {
            title,
            widgets,
            button_states,
            text_states,
            closed: false,
        }
    }

    /// 将控件状态写回 VM 的栈中
    /// 
    /// 按钮: 点击过则压入 "1"，否则压入 "0"
    /// 文本输入: 将输入值压入栈
    pub fn push_to_stacks(&self, stacks: &mut HashMap<String, Vec<String>>) {
        for widget in &self.widgets {
            match widget {
                GuiWidget::Button { var_name, .. } => {
                    let clicked = self.button_states.get(var_name).map(|s| s.clicked).unwrap_or(false);
                    let val = if clicked { "1" } else { "0" };
                    stacks
                        .entry(var_name.clone())
                        .or_insert_with(Vec::new)
                        .push(val.to_string());
                }
                GuiWidget::TextInput { var_name, .. } => {
                    let value = self.text_states.get(var_name).map(|s| s.value.clone()).unwrap_or_default();
                    stacks
                        .entry(var_name.clone())
                        .or_insert_with(Vec::new)
                        .push(value);
                }
                _ => {}
            }
        }
    }
}

// ============================================================
// egui App 实现
// ============================================================

/// egui 应用程序
pub struct QuernGuiApp {
    state: Arc<Mutex<GuiAppState>>,
}

impl QuernGuiApp {
    pub fn new(state: Arc<Mutex<GuiAppState>>) -> Self {
        QuernGuiApp { state }
    }
}

impl eframe::App for QuernGuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            // 窗口标题
            {
                let state = self.state.lock().unwrap();
                ui.heading(&state.title);
            }
            ui.separator();

            // 克隆控件列表以释放锁，避免后续 egui 调用期间持有锁
            let widgets = {
                let state = self.state.lock().unwrap();
                state.widgets.clone()
            };

            // 渲染每个控件
            for widget in &widgets {
                match widget {
                    GuiWidget::Label { text } => {
                        ui.label(text);
                        ui.separator();
                    }
                    GuiWidget::Button { label, var_name } => {
                        let response = ui.button(label);
                        
                        if response.clicked() {
                            let mut state_mut = self.state.lock().unwrap();
                            if let Some(btn) = state_mut.button_states.get_mut(var_name) {
                                btn.clicked = true;
                            }
                        }
                        
                        ui.separator();
                    }
                    GuiWidget::TextInput { placeholder, var_name } => {
                        // 先读取当前文本值（在块内释放锁）
                        let current_value = {
                            let state = self.state.lock().unwrap();
                            state.text_states.get(var_name)
                                .map(|s| s.value.clone())
                                .unwrap_or_default()
                        };
                        
                        let mut text_value = current_value;
                        
                        // 使用 placeholder 作为初始值（如果为空）
                        if text_value.is_empty() {
                            text_value = placeholder.clone();
                        }
                        
                        let _response = ui.text_edit_singleline(&mut text_value);
                        
                        // 更新状态（重新获取锁）
                        let mut state_mut = self.state.lock().unwrap();
                        if let Some(existing) = state_mut.text_states.get_mut(var_name) {
                            existing.value = text_value;
                        }
                        
                        ui.separator();
                    }
                }
            }
        });
    }
}

// ============================================================
// 公共接口：运行 GUI 窗口（阻塞直到窗口关闭）
// ============================================================

/// 在 QB 虚拟机中执行 GUI 指令时调用此函数
/// 
/// # 参数
/// * `title` - 窗口标题
/// * `widgets` - 窗口中的控件列表
/// * `stacks` - VM 的栈集合引用，控件状态将写回此处
/// 
/// # 行为
/// 此函数会阻塞当前线程，直到用户关闭 GUI 窗口。
/// 窗口关闭后，所有控件的状态会被写回 VM 的栈中。
/// 
/// # 错误处理
/// 如果 GUI 初始化失败（如没有显示服务器），会打印错误信息并返回，
/// 不会 panic，程序可以继续执行后续指令。
pub fn run_gui_window(
    title: String,
    widgets: Vec<GuiWidget>,
    stacks: &mut HashMap<String, Vec<String>>,
) {
    // 创建共享状态
    let app_state = Arc::new(Mutex::new(GuiAppState::new(title, widgets)));
    let app_state_clone = app_state.clone();

    // 创建 eframe 选项
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Quern GUI")
            .with_inner_size([400.0, 300.0]),
        ..Default::default()
    };

    // 运行 egui 应用（此调用会阻塞直到窗口关闭）
    // eframe::run_native 签名: run_native(app_name, options, app_creator)
    // app_creator: |&CreationContext| -> Result<Box<dyn App>>
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        eframe::run_native(
            "QuernGUI",
            options,
            Box::new(|_cc| Ok(Box::new(QuernGuiApp::new(app_state_clone)) as Box<dyn eframe::App>)),
        )
    }));

    match result {
        Ok(Ok(())) => {
            // 正常关闭
        }
        Ok(Err(e)) => {
            eprintln!("[Quern GUI Error] run_native returned error: {}", e);
        }
        Err(panic_info) => {
            let msg = if let Some(s) = panic_info.downcast_ref::<&str>() {
                s.to_string()
            } else if let Some(s) = panic_info.downcast_ref::<String>() {
                s.clone()
            } else {
                "Unknown panic".to_string()
            };
            eprintln!("[Quern GUI Error] Panic: {}", msg);
        }
    };

    // 窗口已关闭，将控件状态写回 VM 栈
    // 使用显式块确保 MutexGuard 在 app_state 之前释放
    {
        let guard = app_state.lock();
        if let Ok(state) = guard {
            state.push_to_stacks(stacks);
            println!("[Quern GUI] Window closed, widget states pushed to stack");
        }
    }
}
