import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

# 获取脚本所在目录，等同于 PowerShell 的 $PSScriptRoot
SCRIPT_DIR = Path(__file__).resolve().parent

def run_command(command, cwd=None, env=None):
    """
    执行外部命令，捕获输出并处理异常
    """
    try:
        print(f"[CMD] {' '.join(command)}")
        result = subprocess.run(
            command,
            cwd=cwd,
            env=env,
            capture_output=True,
            text=True,
            encoding='utf-8'
        )
        if result.returncode != 0:
            print(f"[ERROR] 命令执行失败 (返回码: {result.returncode})")
            print(f"[STDOUT] {result.stdout}")
            print(f"[STDERR] {result.stderr}")
            raise subprocess.CalledProcessError(result.returncode, command)
        return result
    except FileNotFoundError:
        print(f"[ERROR] 找不到命令: {command[0]}，请检查环境配置。")
        sys.exit(1)

def safe_move(source, destination):
    """
    安全地移动文件，自动创建目标目录
    """
    src_path = Path(source)
    dst_path = Path(destination)
    
    if not src_path.exists():
        print(f"[WARNING] 源文件不存在，跳过移动: {src_path}")
        return

    dst_path.parent.mkdir(parents=True, exist_ok=True)
    print(f"[MOVE] {src_path} -> {dst_path}")
    shutil.move(str(src_path), str(dst_path))

def invoke_build_step(name, func):
    """
    构建步骤包装器，提供统一的日志输出和异常捕获
    """
    print(f"\n>> 开始: {name}")
    try:
        func()
        print(f">> 完成: {name}")
    except Exception as e:
        print(f"[FAILED] {name} : {e}")
        # 根据需求，这里可以选择是否 sys.exit(1) 中断整个构建
        # 原 PowerShell 脚本在 catch 后继续执行，这里保持一致

def load_config():
    """
    读取并解析 build.json 配置文件
    """
    config_path = SCRIPT_DIR / "build.json"
    if not config_path.exists():
        print(f"[ERROR] 找不到配置文件: {config_path}")
        sys.exit(1)
    
    with open(config_path, 'r', encoding='utf-8') as f:
        return json.load(f)

def main():
    # 解析命令行参数 (模拟 PowerShell 的 param)
    version = 1
    if len(sys.argv) > 1 and sys.argv[1].isdigit():
        version = int(sys.argv[1])

    # 1. 读取配置
    config = load_config()
    paths = config['paths']
    tools = config['tools']
    targets = config['targets']

    # 将路径转换为 Path 对象
    root_path = Path(paths['root'])
    template_path = Path(paths['template'])
    versions_path = Path(paths['versions'])
    output_path = Path(paths['output'])

    # 切换到项目根目录
    os.chdir(root_path)

    # --- Rust 部分 ---
    def build_rust_windows():
        rust_bin = tools['rust']
        target_dir = root_path / "target" / "release"
        main_rs = root_path / "src" / "main.rs"

        # Build 1: Qvm
        if (target_dir / "Quern_cargo.exe").exists():
            (target_dir / "Quern_cargo.exe").unlink()
        for d_file in target_dir.glob("*.d"):
            d_file.unlink()
            
        main_rs.write_text('include!("qvm/main.rs");', encoding='utf-8')
        print(f"[INFO] main.rs 内容: {main_rs.read_text(encoding='utf-8')}")
        print("[INFO] Removed cache files of last build")
        
        run_command([rust_bin, "build", "--release"])
        safe_move(target_dir / "Quern_cargo.exe", template_path / "Windows" / "Qvm.exe")

        # Build 2: Quern
        print("[INFO] Switching to Quern Launcher build target...")
        main_rs.write_text('include!("logger/main.rs");', encoding='utf-8')
        print(f"[INFO] main.rs 内容: {main_rs.read_text(encoding='utf-8')}")
        
        if (target_dir / "Quern_cargo.exe").exists():
            (target_dir / "Quern_cargo.exe").unlink()
        for d_file in target_dir.glob("*.d"):
            d_file.unlink()
        print("[INFO] Removed cache files of Qvm")
        
        run_command([rust_bin, "build", "--release"])
        safe_move(target_dir / "Quern_cargo.exe", template_path / "Windows" / "Quern.exe")

    def build_rust_linux():
        target_dir = root_path / "target" / "release"
        # 转换 Windows 路径到 WSL 路径
        wsl_target_dir = str(target_dir).replace("\\", "/").replace(":", "")
        wsl_target_dir = f"/mnt/{wsl_target_dir[0].lower()}{wsl_target_dir[1:]}"

        # Build 1: Qvm
        run_command(["wsl", "rm", "-f", f"{wsl_target_dir}/Quern_cargo.exe"])
        run_command(["wsl", "find", wsl_target_dir, "-name", "*.d", "-delete"])
        
        print("[INFO] Building Qvm...")
        run_command(["wsl", "cargo", "build", "--release"])
        safe_move(target_dir / "Quern_cargo.exe", template_path / "Windows" / "Qvm.exe")

        # Build 2: Quern
        print("[INFO] Switching to Quern Launcher build target...")
        run_command(["wsl", "rm", "-f", f"{wsl_target_dir}/Quern_cargo.exe"])
        run_command(["wsl", "find", wsl_target_dir, "-name", "*.d", "-delete"])
        
        print("[INFO] Building Quern...")
        run_command(["wsl", "cargo", "build", "--release"])
        safe_move(target_dir / "Quern_cargo.exe", template_path / "Windows" / "Quern.exe")

    if targets.get('rust_windows'):
        invoke_build_step("Rust (Windows) Quern Launcher & Qvm", build_rust_windows)

    if targets.get('rust_linux'):
        invoke_build_step("Rust (Linux) Quern Launcher & Qvm", build_rust_linux)

    # --- C++ AOT 部分 ---
    def build_cpp_windows():
        run_command([
            "clang++", "src\\aot\\main.cpp", "-o", "QuernBuild.exe",
            "-std=c++17", "-static", "-O3", "-s", "-Wall", "-Wextra",
            "-Wno-unused-parameter", "-Wno-unused-variable", "-Wno-unused-function"
        ])
        safe_move(root_path / "QuernBuild.exe", template_path / "Windows" / "QuernBuild.exe")

    def build_cpp_linux():
        run_command([
            "wsl", "g++", "/mnt/i/Quern/QuernProj/Quern/src/aot/main.cpp",
            "-o", "/mnt/i/Quern/QuernProj/Quern/QuernBuild",
            "-std=c++17", "-static", "-O3", "-Wall", "-Wextra",
            "-Wno-unused-parameter", "-Wno-unused-variable", "-Wno-unused-function",
            "-Wno-unused-but-set-variable", "-g1", "-fno-omit-frame-pointer"
        ])
        safe_move(root_path / "QuernBuild", template_path / "Linux" / "QuernBuild")

    if targets.get('cpp_windows'):
        invoke_build_step("C++ AOT (Windows)", build_cpp_windows)

    if targets.get('cpp_linux'):
        invoke_build_step("C++ AOT (Linux)", build_cpp_linux)

    # --- Go 翻译机部分 ---
    os.chdir(root_path / "src" / "translator")
    go_bin = tools['go']

    def build_go_windows():
        env = os.environ.copy()
        env.update({"CGO_ENABLED": "0", "GOOS": "windows", "GOARCH": "amd64"})
        run_command([
            go_bin, "build", "-trimpath", 
            "-ldflags=-s -w -buildmode=exe", 
            "-o", "Quernc.exe", "."
        ], env=env)
        safe_move("Quernc.exe", template_path / "Windows" / "Quernc.exe")

    def build_go_linux():
        env = os.environ.copy()
        env.update({"CGO_ENABLED": "0", "GOOS": "linux", "GOARCH": "amd64"})
        run_command([
            go_bin, "build", "-trimpath", 
            "-ldflags=-s -w", 
            "-o", "Quernc-linux", "tnstr/translator.go"
        ], env=env)
        safe_move("Quernc-linux", template_path / "Linux" / "Quernc")

    def build_go_macos_arm64():
        env = os.environ.copy()
        env.update({"CGO_ENABLED": "0", "GOOS": "darwin", "GOARCH": "arm64"})
        run_command([
            go_bin, "build", "-trimpath", 
            "-ldflags=-s -w", 
            "-o", "Quernc-macOS-arm64", "tnstr/translator.go"
        ], env=env)
        safe_move("Quernc-macOS-arm64", template_path / "macOS" / "arm64" / "Quernc")

    if targets.get('go_windows'):
        invoke_build_step("Go (Windows)", build_go_windows)
    if targets.get('go_linux'):
        invoke_build_step("Go (Linux)", build_go_linux)
    if targets.get('go_macos_arm64'):
        invoke_build_step("Go (macOS ARM64)", build_go_macos_arm64)

    # --- QLM 部分 ---
    os.chdir(root_path)
    def build_qlm_windows():
        run_command([
            "clang++", "src\\qlm\\main.cpp", "-o", "Qlm.exe",
            "-std=c++17", "-static", "-O3", "-s", "-Wall", "-Wextra",
            "-Wno-unused-parameter", "-Wno-unused-variable", "-Wno-unused-function"
        ])
        safe_move(root_path / "Qlm.exe", template_path / "Windows" / "Qlm.exe")

    def build_qlm_linux():
        run_command([
            "wsl", "g++", "/mnt/i/Quern/QuernProj/Quern/src/qlm/main.cpp",
            "-o", "/mnt/i/Quern/QuernProj/Quern/QuernBuild",
            "-std=c++17", "-static", "-O3", "-Wall", "-Wextra",
            "-Wno-unused-parameter", "-Wno-unused-variable", "-Wno-unused-function",
            "-Wno-unused-but-set-variable", "-g1", "-fno-omit-frame-pointer"
        ])
        safe_move(root_path / "Qlm", template_path / "Linux" / "Qlm")

    if targets.get('qlm_windows'):
        invoke_build_step("QLM (Windows)", build_qlm_windows)
    if targets.get('qlm_linux'):
        invoke_build_step("QLM (Linux)", build_qlm_linux)

    # --- Update 部分 ---
    def build_update_windows():
        run_command([
            "clang++", "src\\update\\main.cpp", "-o", "Updater.exe",
            "-std=c++17", "-static", "-O3", "-s", "-Wall", "-Wextra",
            "-Wno-unused-parameter", "-Wno-unused-variable", "-Wno-unused-function"
        ])
        safe_move(root_path / "Updater.exe", template_path / "Windows" / "Updater.exe")

    def build_update_linux():
        run_command([
            "wsl", "g++", "/mnt/i/Quern/QuernProj/Quern/src/update/main.cpp",
            "-o", "/mnt/i/Quern/QuernProj/Quern/Updater",
            "-std=c++17", "-static", "-O3", "-Wall", "-Wextra",
            "-Wno-unused-parameter", "-Wno-unused-variable", "-Wno-unused-function",
            "-Wno-unused-but-set-variable", "-g1", "-fno-omit-frame-pointer"
        ])
        safe_move(root_path / "Updater", template_path / "Linux" / "Updater")

    if targets.get('update_windows'):
        invoke_build_step("Updater (Windows)", build_update_windows)
    if targets.get('update_linux'):
        invoke_build_step("Updater (Linux)", build_update_linux)

    # --- 修复后的 QVM 重命名逻辑 ---
    os.chdir(root_path)
    old_qvm_path = template_path / "Qvm.exe"
    source_exe_path = template_path / "Quern_cargo.exe"

    if old_qvm_path.exists():
        print("[INFO] Removing old Qvm.exe...")
        old_qvm_path.unlink()
    else:
        print("[INFO] No old Qvm.exe to remove.")

    if source_exe_path.exists():
        print("[MOVE] Renaming Quern_cargo.exe -> Qvm.exe")
        try:
            shutil.move(str(source_exe_path), str(old_qvm_path))
            print("[SUCCESS] Qvm.exe created successfully.")
        except Exception as e:
            print(f"[ERROR] Failed to move/rename Quern_cargo.exe: {e}")
    else:
        print(f"[WARNING] Quern_cargo.exe not found at {source_exe_path}, skipping Qvm rename.")

    # --- 打包与安装程序 ---
    def pack_zip():
        zip_path = root_path / "Quern-Linux&Windows-amd64-LSDK.zip"
        run_command([tools['seven_zip'], "a", str(zip_path), f"{template_path}\\*"])
        run_command([tools['seven_zip'], "d", str(zip_path), "setup.iss"])
        dest_zip = versions_path / str(version) / zip_path.name
        safe_move(zip_path, dest_zip)

    def pack_installer():
        run_command([tools['inno_setup'], f"/O{output_path}", f"{template_path}\\setup.iss"])
        installer_path = output_path / "Quern-Windows-amd64-LSDK_Installer.exe"
        dest_installer = versions_path / str(version) / installer_path.name
        safe_move(installer_path, dest_installer)

    if targets.get('pack_zip'):
        invoke_build_step("打包 ZIP", pack_zip)

    if targets.get('pack_installer'):
        invoke_build_step("生成 Inno Setup 安装包", pack_installer)

    # --- 最终部署 ---
    try:
        shutil.copytree("I:\\Quern\\Quern_Template\\Windows", "I:\\VW\\vw_sys\\user\\deft\\Quern", dirs_exist_ok=True)
        shutil.copy2("I:\\Quern\\Quern_Template\\Windows\\Updater.exe", "I:\\VW\\vw_sys\\user\\deft\\Quern")
    except Exception as e:
        print(f"[WARNING] 最终部署复制失败: {e}")

    print(f"\n[SUCCESS] 所有配置驱动的任务已完成！版本: {version}")
    if (versions_path / str(version)).exists():
        for item in (versions_path / str(version)).iterdir():
            print(f" - {item.name}")

if __name__ == "__main__":
    main()