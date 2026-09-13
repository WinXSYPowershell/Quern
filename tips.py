import time
import datetime
import psutil
import threading
import sys
import os

# 尝试导入tkinter用于全屏遮罩
try:
    import tkinter as tk
    from tkinter import font
    TKINTER_AVAILABLE = True
except ImportError:
    TKINTER_AVAILABLE = False
    print("警告: 未找到tkinter，将无法显示强制全屏遮罩，仅会有声音提示(如果系统支持)或控制台提示。")

class EyeCareMonitor:
    def __init__(self, start_hour=8, start_minute=0, end_hour=21, end_minute=0, interval_minutes=40):
        self.start_time = datetime.datetime.now().replace(hour=start_hour, minute=start_minute, second=0, microsecond=0)
        self.end_time = datetime.datetime.now().replace(hour=end_hour, minute=end_minute, second=0, microsecond=0)
        
        # 如果当前时间已经过了今天的开始时间，可能需要调整逻辑，这里假设用户会在8点前运行或刚过8点
        # 为了演示方便，如果当前时间晚于start_time，我们仍然以今天的start_time为基准计算
        
        self.interval = datetime.timedelta(minutes=interval_minutes)
        self.rest_duration_seconds = 120  # 强制休息2分钟
        
        # 计算所有提醒时间点
        self.reminder_times = []
        current_check = self.start_time
        while current_check <= self.end_time:
            self.reminder_times.append(current_check)
            current_check += self.interval
            
        # 确保结束时间点也在提醒列表中（如果不在的话）
        if self.end_time not in self.reminder_times:
            self.reminder_times.append(self.end_time)
            
        self.reminder_times.sort()
        
        self.is_resting = False
        self.next_reminder = self._get_next_reminder()

    def _get_next_reminder(self):
        now = datetime.datetime.now()
        for t in self.reminder_times:
            if t > now:
                return t
        return None

    def get_system_stats(self):
        cpu_percent = psutil.cpu_percent(interval=1) # 阻塞1秒获取准确CPU，或者用interval=None非阻塞但可能不准
        memory = psutil.virtual_memory()
        disk = psutil.disk_usage('/')
        
        mem_percent = memory.percent
        disk_percent = disk.percent
        
        return cpu_percent, mem_percent, disk_percent

    def show_rest_overlay(self):
        """显示全屏遮罩"""
        if not TKINTER_AVAILABLE:
            print("\n>>> 请休息眼睛！(无法启动图形遮罩) <<<")
            time.sleep(self.rest_duration_seconds)
            return

        def run_overlay():
            root = tk.Tk()
            root.attributes('-fullscreen', True)
            root.attributes('-topmost', True)
            root.configure(bg='black')
            
            # 创建关闭按钮或自动关闭逻辑
            # 这里我们做一个简单的标签
            label_frame = tk.Frame(root, bg='black')
            label_frame.place(relx=0.5, rely=0.5, anchor='center')
            
            msg = "护眼时间\n请远眺或闭眼休息\n"
            if datetime.datetime.now().hour == self.end_time.hour and datetime.datetime.now().minute >= self.end_time.minute - 1:
                 msg += "21:00了！记得敷眼睛哦！\n"
            
            label = tk.Label(label_frame, text=msg, fg='white', bg='black', font=('Microsoft YaHei', 40, 'bold'))
            label.pack(pady=20)
            
            timer_label = tk.Label(label_frame, text=f"剩余时间: {self.rest_duration_seconds}s", fg='#00FF00', bg='black', font=('Consolas', 20))
            timer_label.pack()

            # 倒计时逻辑
            remaining = self.rest_duration_seconds
            def update_timer():
                nonlocal remaining
                if remaining > 0:
                    timer_label.config(text=f"剩余时间: {remaining}s")
                    remaining -= 1
                    root.after(1000, update_timer)
                else:
                    root.destroy()
            
            root.after(1000, update_timer)
            
            # 允许按ESC退出（以防万一程序卡死）
            root.bind('<Escape>', lambda e: root.destroy())
            
            root.mainloop()

        # 在新线程中运行GUI，以免阻塞主进程的其他逻辑（虽然休息时主进程也没啥事做）
        gui_thread = threading.Thread(target=run_overlay, daemon=True)
        gui_thread.start()
        
        # 主线程等待GUI线程结束，或者简单sleep
        # 由于daemon=True，主线程如果不等待，可能会直接退出导致窗口消失
        # 所以这里主线程必须阻塞
        gui_thread.join()

    def run(self):
        print("="*30 + " 护眼监控启动 " + "="*30)
        print(f"工作时间: {self.start_time.strftime('%H:%M')} - {self.end_time.strftime('%H:%M')}")
        print(f"休息间隔: 每40分钟")
        print(f"下次休息: {self.next_reminder.strftime('%H:%M') if self.next_reminder else '无'}")
        print("按 Ctrl+C 退出")
        print("-" * 70)

        try:
            while True:
                now = datetime.datetime.now()
                
                # 检查是否结束
                if now >= self.end_time and not self.is_resting:
                    # 如果是9点整，触发最后一次提醒
                    if now.hour == self.end_time.hour and now.minute == self.end_time.minute and now.second < 2:
                         self.trigger_rest("9:00了！请敷眼睛！")
                    elif now > self.end_time + datetime.timedelta(minutes=5):
                        # 超过结束时间5分钟，自动退出
                        print("\n工作时间已结束，程序退出。")
                        break

                # 检查是否到达休息时间
                if self.next_reminder and now >= self.next_reminder and not self.is_resting:
                    self.trigger_rest()

                # 获取系统信息 (使用interval=None避免每次阻塞1秒，导致界面刷新慢，改为粗略估计或接受1秒延迟)
                # 为了控制台刷新流畅，我们这里不使用psutil.cpu_percent(interval=1)，而是使用非阻塞模式
                cpu_percent = psutil.cpu_percent(interval=None)
                memory = psutil.virtual_memory()
                disk = psutil.disk_usage('/')
                
                mem_percent = memory.percent
                disk_percent = disk.percent
                
                # 格式化输出
                current_time_str = now.strftime("%H:%M:%S")
                next_reminder_str = self.next_reminder.strftime("%H:%M") if self.next_reminder else "已完成"
                
                # 构建状态栏字符串
                status_line = (
                    f"\r[时间: {current_time_str}] "
                    f"[下次护眼: {next_reminder_str}] "
                    f"[CPU: {cpu_percent:>5.1f}%] "
                    f"[内存: {mem_percent:>5.1f}%] "
                    f"[磁盘: {disk_percent:>5.1f}%] "
                )
                
                sys.stdout.write(status_line)
                sys.stdout.flush()
                
                time.sleep(1) # 每秒更新一次
                
        except KeyboardInterrupt:
            print("\n\n程序已手动停止。")

    def trigger_rest(self, custom_msg=None):
        self.is_resting = True
        msg = custom_msg if custom_msg else "时间到！请护眼休息！"
        print(f"\n\n{'!'*20} {msg} {'!'*20}")
        print(f"强制全屏休息 {self.rest_duration_seconds} 秒...")
        
        self.show_rest_overlay()
        
        self.is_resting = False
        # 更新下一次提醒时间
        self.next_reminder = self._get_next_reminder()
        print(f"\n休息结束。下次护眼时间: {self.next_reminder.strftime('%H:%M') if self.next_reminder else '无'}")
        print("-" * 70)

if __name__ == "__main__":
    monitor = EyeCareMonitor()
    monitor.run()
