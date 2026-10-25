// ============================================================
// QVM FFI 使用示例
// 编译命令（假设 Rust 库名为 libqvm.so）:
//   g++ -std=c++17 ffi_example.cpp -o ffi_example -lqvm -lpthread
// ============================================================

#include <iostream>
#include <cstring>
#include <cstdlib>
#include "qvm.h"

// ============================================================
// 示例 1: 实现一个简单的数学加法函数（作为 C++ 回调）
// ============================================================

extern "C" const char* math_add(const char** args, int arg_count) {
    if (arg_count < 2) {
        return "Error: math.add requires 2 arguments";
    }
    // 简单解析两个数字并相加
    double a = std::strtod(args[0], nullptr);
    double b = std::strtod(args[1], nullptr);
    double result = a + b;

    // 分配内存返回结果（QVM 侧会 free）
    char buf[64];
    snprintf(buf, sizeof(buf), "%g", result);
    return strdup(buf);
}

extern "C" const char* math_multiply(const char** args, int arg_count) {
    if (arg_count < 2) {
        return "Error: math.multiply requires 2 arguments";
    }
    double a = std::strtod(args[0], nullptr);
    double b = std::strtod(args[1], nullptr);
    double result = a * b;

    char buf[64];
    snprintf(buf, sizeof(buf), "%g", result);
    return strdup(buf);
}

// ============================================================
// 示例 2: 实现一个系统命令执行函数
// ============================================================

extern "C" const char* sys_exec(const char** args, int arg_count) {
    if (arg_count < 1) {
        return "Error: sys.exec requires 1 argument";
    }
    // 拼接参数
    std::string cmd = args[0];
    for (int i = 1; i < arg_count; ++i) {
        cmd += " " + std::string(args[i]);
    }
    // 执行命令（生产环境应做严格的安全校验）
    FILE* pipe = popen(cmd.c_str(), "r");
    if (!pipe) {
        return "Error: failed to execute command";
    }
    char buffer[256];
    std::string result;
    while (fgets(buffer, sizeof(buffer), pipe) != nullptr) {
        result += buffer;
    }
    pclose(pipe);
    return strdup(result.empty() ? "(empty)" : result.c_str());
}

// ============================================================
// 示例 3: 完整的集成流程
// ============================================================

int main() {
    std::cout << "=== QVM FFI 使用示例 ===" << std::endl;

    // 1. 创建 QVM 实例
    qvm_handle_t vm = qvm_new();
    if (!vm) {
        std::cerr << "Failed to create QVM instance" << std::endl;
        return 1;
    }

    // 2. 注册宿主函数（使用自定义回调）
    std::cout << "\n[1] 注册宿主函数..." << std::endl;
    qvm_register_function_callback(vm, "Math", "add", math_add);
    qvm_register_function_callback(vm, "Math", "multiply", math_multiply);
    qvm_register_function_callback(vm, "Sys", "exec", sys_exec);
    std::cout << "    已注册: Math.add, Math.multiply, Sys.exec" << std::endl;

    // 3. 注册宿主变量
    std::cout << "\n[2] 注册宿主变量..." << std::endl;
    qvm_register_variable(vm, "APP_NAME", "QVMDemo");
    qvm_register_variable(vm, "VERSION", "1.0.0");
    std::cout << "    已注册: APP_NAME=QVMDemo, VERSION=1.0.0" << std::endl;

    // 4. 执行 QVM 脚本命令
    std::cout << "\n[3] 执行 QVM 命令..." << std::endl;

    // 创建栈
    qvm_run_command(vm, "crt my_stack");

    // 使用 host_call 调用注册的宿主函数
    qvm_run_command(vm, "host_call Math.add 10 20");
    // 预期输出: [HostCall: Math.add] 30

    qvm_run_command(vm, "host_call Math.multiply 6 7");
    // 预期输出: [HostCall: Math.multiply] 42

    // 调用系统命令（注意安全风险）
    qvm_run_command(vm, "host_call Sys.exec echo Hello from QVM");
    // 预期输出: [HostCall: Sys.exec] Hello from QVM

    // 5. 列出已注册模块
    std::cout << "\n[4] 列出已注册模块..." << std::endl;
    char** modules = qvm_list_modules(vm);
    if (modules) {
        for (int i = 0; modules[i] != nullptr; ++i) {
            std::cout << "    模块 " << i << ": " << modules[i] << std::endl;
        }
        qvm_free_modules(modules);
    }

    // 6. 检查错误
    const char* error = qvm_get_last_error(vm);
    if (error) {
        std::cout << "\n[!] 最后错误: " << error << std::endl;
    } else {
        std::cout << "\n[OK] 无错误" << std::endl;
    }

    // 7. 释放资源
    std::cout << "\n[5] 释放 QVM 实例..." << std::endl;
    qvm_free(vm);
    std::cout << "    QVM 已释放" << std::endl;

    return 0;
}

// ============================================================
// 示例 4: 执行脚本文件的方式
// ============================================================
/*
 * 你可以将 QVM 脚本写入文件，然后用 qvm_run_script 执行:
 *
 * // script.qvm 内容:
 * //   crt result
 * //   host_call Math.add 100 200
 * //   out result
 * //   otn
 *
 * int main_script() {
 *     qvm_handle_t vm = qvm_new();
 *     qvm_register_function_callback(vm, "Math", "add", math_add);
 *
 *     int ret = qvm_run_script(vm, "script.qvm");
 *     if (ret == 0) {
 *         std::cout << "脚本执行成功" << std::endl;
 *     } else {
 *         std::cout << "脚本执行失败: " << qvm_get_last_error(vm) << std::endl;
 *     }
 *
 *     qvm_free(vm);
 *     return ret;
 * }
 */
