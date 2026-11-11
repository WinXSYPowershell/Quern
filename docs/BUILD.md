# BUILD.md — Quern 工具链从源码编译指南

> 本文件说明如何用仓库自带的 **`build.py`** 一键编译 Quern 工具链的全部组件，并生成发布 ZIP / Windows 安装包。
>
> 工具链由 6 个组件构成，分布在 4 种语言里：
>
> | 组件 | 语言 | 源码入口 | 构建产物 |
> |------|------|----------|----------|
> | **Quernc**（翻译器） | Go | `src/translator/` | `Quernc[.exe]` |
> | **Qvm**（虚拟机） | Rust | `src/qvm/main.rs` | `Qvm[.exe]` |
> | **Quern**（Launcher） | Rust | `src/logger/main.rs` | `Quern[.exe]` |
> | **QuernBuild**（AOT 编译器） | C++17 | `src/aot/main.cpp` | `QuernBuild[.exe]` |
> | **Qlm**（模块管理器） | C++17 | `src/qlm/main.cpp` | `Qlm[.exe]` |
> | **Updater**（自动更新器） | C++17 | `src/update/main.cpp` | `Updater[.exe]` |

---

## 1. 架构前提

`build.py` 采用 **「Windows 主控 + WSL 交叉构建」** 模式：

```
┌──────────────────────────────────────────────┐
│  Windows 主机（运行 build.py 的地方）           │
│  ┌────────────────────────────────────────┐   │
│  │  clang++     → Windows 版 C++ 组件      │   │
│  │  go          → Windows 版 Quernc        │   │
│  │  cargo       → Windows 版 Qvm/Quern     │   │
│  └────────────────────────────────────────┘   │
│  ┌────────────────────────────────────────┐   │
│  │  wsl ─┬─ g++      → Linux 版 C++ 组件   │   │
│  │       ├─ cargo    → Linux 版 Qvm/Quern  │   │
│  │       └─ （Go 用 GOOS=linux 交叉编译）   │   │
│  └────────────────────────────────────────┘   │
│  7z 打包  →  Inno Setup 打包（Windows）        │
└──────────────────────────────────────────────┘
```

>  **重要**：`build.py` 中大量使用了硬编码路径 `I:\Quern\QuernProj\Quern\` 与 WSL 路径 `/mnt/i/Quern/QuernProj/Quern/`（见 §6）。**在其他机器上第一次使用前必须先按 §6 修改这些路径**，否则 Linux 构建会失败。

如果你只想在 Linux 本机构建，可以只跑 Go 那一节（`GOOS=linux go build`），或参考 §7 手动构建。

---

## 2. 环境要求

### 2.1 必需软件（Windows 主机）

| 工具 | 用途 | 备注 |
|------|------|------|
| **Python 3.8+** | 运行 `build.py` | 只需标准库，无第三方依赖 |
| **clang++** | 编译 Windows 版 C++ 组件 | 必须在 `PATH` 中；建议 LLVM 官方 installer |
| **Go 1.20+** | 编译 Quernc | 建议设 `GOPROXY=https://goproxy.cn,direct`（国内） |
| **Rust (rustup + cargo)** | 编译 Qvm / Quern | `rustup target add x86_64-pc-windows-msvc` 已默认 |
| **WSL2 + 一个 Linux 发行版** | 交叉构建 Linux 产物 | Ubuntu 20.04+ 推荐 |
| **7-Zip** (`7z.exe`) | `pack_zip` | 一般在 `C:\Program Files\7-Zip\7z.exe` |
| **Inno Setup 6** (`ISCC.exe`) | `pack_installer` | 官方安装器即可 |

### 2.2 WSL 内需要的工具

```bash
# 在 WSL 中执行一次
sudo apt update
sudo apt install -y build-essential g++ cargo
# cargo 若版本太旧可改用 rustup
```

> 注意：`build.py` 的 Linux C++ 构建直接调用 `wsl g++`，**不**会调用 WSL 里的 clang。如果你更偏好 clang，请自行改脚本或用 §7 手动构建。

### 2.3 一次性自检

```bash
python --version
clang++ --version
go version
cargo --version
wsl --version
wsl g++ --version
```

---

## 3. `build.json` 配置详解

`build.py` 的全部行为由同目录下的 `build.json` 驱动。结构如下：

```json
{
  "paths": {
    "root":     "I:\\Quern\\QuernProj\\Quern",
    "template": "I:\\Quern\\Quern_Template",
    "versions": "I:\\Quern\\Versions",
    "output":   "I:\\Quern\\Installer_Output"
  },
  "tools": {
    "rust":        "cargo",
    "go":          "go",
    "seven_zip":   "C:\\Program Files\\7-Zip\\7z.exe",
    "inno_setup":  "C:\\Program Files (x86)\\Inno Setup 6\\ISCC.exe"
  },
  "targets": {
    "rust_windows":    true,
    "rust_linux":      true,
    "cpp_windows":     true,
    "cpp_linux":       true,
    "go_windows":      true,
    "go_linux":        true,
    "go_macos_arm64":  false,
    "qlm_windows":     true,
    "qlm_linux":       true,
    "update_windows":  true,
    "update_linux":    true,
    "pack_zip":        false,
    "pack_installer":  false
  }
}
```

### 3.1 `paths`

| 字段 | 含义 |
|------|------|
| `root` | 仓库根目录（`build.py` 会 `chdir` 到这里） |
| `template` | 产物暂存目录。**每个平台的子目录名固定**：`Windows/`、`Linux/`、`macOS/arm64/` |
| `versions` | 发布产物的归档目录，最终放到 `versions/<version>/` |
| `output` | Inno Setup 中间产物目录 |

### 3.2 `tools`

- `rust` / `go`：可以是命令名（依赖 `PATH`），也可以是绝对路径。若你的 cargo 来自 `rustup` 但在非标准位置，写绝对路径更稳。
- `seven_zip` / `inno_setup`：**必须**是绝对路径。

### 3.3 `targets`

每一位控制一个构建阶段是否执行。按需开/关可以显著加速迭代：

| target | 产物 |
|--------|------|
| `rust_windows` | `Windows/Qvm.exe`、`Windows/Quern.exe` |
| `rust_linux` | （目前脚本把 Linux 产物也放到 `Windows/`，见 §6 注意事项） |
| `cpp_windows` | `Windows/QuernBuild.exe` |
| `cpp_linux` | `Linux/QuernBuild` |
| `go_windows` | `Windows/Quernc.exe` |
| `go_linux` | `Linux/Quernc` |
| `go_macos_arm64` | `macOS/arm64/Quernc` |
| `qlm_windows` | `Windows/Qlm.exe` |
| `qlm_linux` | `Linux/Qlm` |
| `update_windows` | `Windows/Updater.exe` |
| `update_linux` | `Linux/Updater` |
| `pack_zip` | `versions/<ver>/Quern-Linux&Windows-amd64-LSDK.zip` |
| `pack_installer` | `versions/<ver>/Quern-Windows-amd64-LSDK_Installer.exe` |

---

## 4. 一键构建

### 4.1 基本用法

```powershell
python build.py 1
```

- 第一个位置参数 `version`（可选，默认 `1`），只影响 `versions/<version>/` 的归档子目录名。
- 整个流程按顺序执行所有 `targets` 中为 `true` 的步骤，某一步失败**会打印错误但继续**，`invoke_build_step` 不主动 `sys.exit`。

### 4.2 典型迭代场景

| 场景 | 建议配置 |
|------|----------|
| 只改 Go 翻译器，快速验证 Windows | 只开 `go_windows` |
| 只改 C++ AOT 逻辑 | 只开 `cpp_windows` |
| 只改 Rust Launcher | 只开 `rust_windows` |
| 完整发布 | 全开 + `pack_zip` + `pack_installer` |

### 4.3 构建流程（伪代码）

```
load_config()
chdir(root)
├─ [rust_windows]  main.rs ← include!("qvm/main.rs");    cargo build --release → Qvm.exe
│                  main.rs ← include!("logger/main.rs"); cargo build --release → Quern.exe
├─ [rust_linux]    通过 wsl 执行相同的两次 cargo build
├─ [cpp_windows]   clang++ src\aot\main.cpp -std=c++17 -static -O3 -s → QuernBuild.exe
├─ [cpp_linux]     wsl g++ /mnt/i/.../src/aot/main.cpp -std=c++17 -static -O3 → Linux/QuernBuild
├─ [go_windows]    chdir(src/translator); CGO_ENABLED=0 GOOS=windows GOARCH=amd64
│                  go build -trimpath -ldflags="-s -w -buildmode=exe" -o Quernc.exe .
├─ [go_linux]      CGO_ENABLED=0 GOOS=linux GOARCH=amd64 go build -o Quernc-linux .
├─ [go_macos_arm64] GOOS=darwin GOARCH=arm64 go build -o Quernc-macOS-arm64 .
├─ [qlm_windows]   clang++ src\qlm\main.cpp → Qlm.exe
├─ [qlm_linux]     wsl g++ ... → Linux/Qlm
├─ [update_windows] clang++ src\update\main.cpp → Updater.exe
├─ [update_linux]   wsl g++ ... → Linux/Updater
├─ 修复后的 Qvm 重命名逻辑（把可能残留的 Quern_cargo.exe 改名为 Qvm.exe）
├─ [pack_zip]       7z a "<ver>.zip" template\*   → 删除 setup.iss → versions/<ver>/
├─ [pack_installer] ISCC.exe /O<output> template\setup.iss → versions/<ver>/
└─ 部署：把 template\Windows\* 复制到硬编码路径 I:\VW\vw_sys\user\deft\Quern
```

---

## 5. 产物布局

构建成功后，`template` 目录大致如下：

```
<template>/
├── Windows/
│   ├── Quernc.exe
│   ├── Qvm.exe
│   ├── Quern.exe          # = Launcher
│   ├── QuernBuild.exe
│   ├── Qlm.exe
│   ├── Updater.exe
│   ├── setup.iss          # Inno Setup 脚本（打包时排除）
│   └── ...其它运行时资源
├── Linux/
│   ├── Quernc
│   ├── QuernBuild
│   ├── Qlm
│   └── Updater
└── macOS/
    └── arm64/
        └── Quernc
```

归档目录：

```
<versions>/<version>/
├── Quern-Linux&Windows-amd64-LSDK.zip
└── Quern-Windows-amd64-LSDK_Installer.exe
```

---

## 6.  硬编码路径清单（首次使用必改）

`build.py` 中有 **5 处**硬编码路径，位于非 `build.json` 控制的逻辑里。如果你不是原开发者（`I:` 盘 + `/mnt/i/...` 布局），**必须修改**：

| 位置 | 原值 | 修改为 |
|------|------|--------|
| `build_cpp_linux()` | `"/mnt/i/Quern/QuernProj/Quern/src/aot/main.cpp"` | 你的仓库 WSL 路径 |
| `build_cpp_linux()` | `"/mnt/i/Quern/QuernProj/Quern/QuernBuild"` | 同上（输出路径） |
| `build_qlm_linux()` | `"/mnt/i/Quern/QuernProj/Quern/src/qlm/main.cpp"` | 同上 |
| `build_update_linux()` | `"/mnt/i/Quern/QuernProj/Quern/src/update/main.cpp"` | 同上 |
| 文件末尾 | `I:\\Quern\\Quern_Template\\Windows` → `I:\\VW\\vw_sys\\user\\deft\\Quern` | 你的部署路径（或删除这段部署逻辑） |

> **建议改动**：把这 5 个路径改为基于 `root_path` 动态生成，例如：
>
> ```python
> wsl_root = "/mnt/" + str(root_path.drive[0]).lower() + str(root_path)[2:].replace("\\", "/")
> aot_src  = f"{wsl_root}/src/aot/main.cpp"
> ```
>
> 这样改完后脚本才能真正跨机器使用。

### 6.1 其他已知小问题

1. **`rust_linux` 产物位置**：`build_rust_linux()` 里 `safe_move(..., template_path / "Windows" / "Qvm.exe")` —— Linux 版产物也被塞到 `Windows/`。如果你在意 Linux 包内 Qvm 是否正确，需要改成 `template_path / "Linux" / ...`。
2. **`qlm_linux` 输出名**：`-o /mnt/i/.../QuernBuild` 生成的文件名为 `QuernBuild`（不是 `Qlm`），随后 `safe_move(root_path / "Qlm", ...)` 会移动一个**不存在的文件**，`safe_move` 只打印 warning 跳过。修复方法：把 `-o` 参数改成 `.../Qlm`。
3. **`pack_zip` 会删除 setup.iss**：`7z d <zip> setup.iss` —— 从已生成的 ZIP 里删掉它，避免 Windows 安装脚本被混入通用包。这是有意的，不用管。
4. **`main.rs` 被覆盖**：Rust 两次构建靠改写 `src/main.rs` 为 `include!("qvm/main.rs");` / `include!("logger/main.rs");` 切换目标。这是工程上取巧的做法：**每次构建会覆盖仓库里的 `src/main.rs`**，Git 状态会变脏。建议构建完成后执行 `git checkout src/main.rs`，或本地保留改动。

---

## 7. 手动构建（不依赖 build.py）

如果你想在任意平台上独立构建各组件：

### 7.1 Quernc（Go）

```bash
cd src/translator

# 本地平台
go build -trimpath -ldflags "-s -w" -o Quernc .

# 交叉编译 Windows
CGO_ENABLED=0 GOOS=windows GOARCH=amd64 \
  go build -trimpath -ldflags "-s -w" -o Quernc.exe .

# 交叉编译 Linux
CGO_ENABLED=0 GOOS=linux GOARCH=amd64 \
  go build -trimpath -ldflags "-s -w" -o Quernc .

# macOS ARM64
CGO_ENABLED=0 GOOS=darwin GOARCH=arm64 \
  go build -trimpath -ldflags "-s -w" -o Quernc-macos-arm64 .
```

### 7.2 Qvm / Quern（Rust）

```bash
# Qvm
cp src/qvm/main.rs src/main.rs
cargo build --release
cp target/release/<crate-name>.exe Qvm.exe   # Windows
cp target/release/<crate-name>     Qvm       # Linux/macOS

# Quern Launcher
cp src/logger/main.rs src/main.rs
cargo build --release
cp target/release/<crate-name>.exe Quern.exe
```

> `<crate-name>` 由 `Cargo.toml` 的 `[[bin]]` 或 `package.name` 决定。`build.py` 里假设它叫 `Quern_cargo`（在 `Cargo.toml` 中配了 `[[bin]] name = "Quern_cargo"`），否则那两个 `safe_move` 会全部因源文件不存在而跳过。

### 7.3 QuernBuild（C++）

```bash
# Windows
clang++ src\aot\main.cpp -o QuernBuild.exe ^
  -std=c++17 -static -O3 -s -Wall -Wextra ^
  -Wno-unused-parameter -Wno-unused-variable -Wno-unused-function

# Linux
g++ src/aot/main.cpp -o QuernBuild \
  -std=c++17 -static -O3 -Wall -Wextra \
  -Wno-unused-parameter -Wno-unused-variable -Wno-unused-function \
  -Wno-unused-but-set-variable -g1 -fno-omit-frame-pointer
```

> `-static` 会让产物不依赖 libstdc++ / libc++ 动态库；如果不想静态链接，去掉 `-static` 即可。

### 7.4 Qlm / Updater（C++）

与 §7.3 一致，把 `src/aot/main.cpp` 换成 `src/qlm/main.cpp` / `src/update/main.cpp` 即可。Updater 在 Windows 上依赖 `crypt32.lib`，用 MSVC 时需要加 `/link crypt32.lib`；用 clang++ 时会通过 `#pragma comment(lib, "crypt32.lib")` 自动链接。

---

## 8. 常见问题

| 症状 | 原因 / 解决 |
|------|-------------|
| `[ERROR] 找不到命令: clang++` | clang 不在 `PATH`；或 `build.json` 中 `tools` 未指定绝对路径 |
| `[ERROR] 命令执行失败` + `wsl: 找不到路径` | 硬编码的 `/mnt/i/...` 路径在你的机器上无效，见 §6 |
| `wsl cargo build` 报 `error: failed to get ...` | WSL 内没装 Rust；`sudo apt install cargo` 或 `curl rustup...` |
| `[WARNING] Quern_cargo.exe not found` | `Cargo.toml` 里 `[[bin]] name` 不是 `Quern_cargo`，改脚本或改 Cargo.toml |
| `cargo build` 报 `missing main.rs` | 前面某次构建改写了 `src/main.rs` 后中断；`git checkout src/main.rs` 恢复 |
| `ISCC.exe` 报错 | Inno Setup 未安装，或 `setup.iss` 路径不正确；注意 `pack_zip` 会在 ZIP 内删掉 `setup.iss` |
| 7z 报 `No files to process` | `template\*` 为空，说明前面构建步骤全部失败或路径配置错误 |
| Go 编译报 `missing go.sum entry` | `cd src/translator && go mod tidy` 后再编译 |
| `Quernc` 找依赖模块失败 | Quernc 运行时会读 `mods/`，构建阶段不需要，运行时才需要 |
| 构建后 `git status` 出现 `src/main.rs` 变更 | 正常现象，见 §6.1(4)；`git checkout src/main.rs` |

---

## 9. 发布 Checklist

一个完整的发布版本建议按以下顺序确认：

- [ ] `build.json` 的 `paths` / `tools` 全部指向正确位置
- [ ] §6 的 5 处硬编码路径已按你的环境改写
- [ ] `targets` 全开（`pack_zip` / `pack_installer` 按需）
- [ ] `git checkout src/main.rs` 确保 Rust 构建前仓库干净
- [ ] 运行 `python build.py <version>`
- [ ] 检查 `template/Windows/` 下 6 个二进制齐全
- [ ] 检查 `template/Linux/` 下 4 个二进制齐全
- [ ] 手工 smoke test：
  ```powershell
  .\template\Windows\Quern.exe --run hello.q
  .\template\Windows\Quern.exe --aot-build hello.q --osize
  ```
- [ ] 确认 `versions/<version>/` 下 ZIP 与 Installer 生成
- [ ] 若使用最终部署逻辑（§6 第 5 条），确认目标路径可写

---

## 10. 组件与构建产物对照表

| 组件 | 源码目录 | 构建命令 | 产物（Windows / Linux / macOS） |
|------|----------|----------|--------------------------------|
| Quernc | `src/translator/` | `go build ...` | `Quernc.exe` / `Quernc` / `Quernc` |
| Qvm | `src/qvm/main.rs` | `cargo build --release` | `Qvm.exe` / `Qvm` / `Qvm` |
| Quern | `src/logger/main.rs` | `cargo build --release` | `Quern.exe` / `Quern` / `Quern` |
| QuernBuild | `src/aot/main.cpp` | `clang++` | `QuernBuild.exe` / `QuernBuild` / — |
| Qlm | `src/qlm/main.cpp` | `clang++` | `Qlm.exe` / `Qlm` / — |
| Updater | `src/update/main.cpp` | `clang++` | `Updater.exe` / `Updater` / — |

---

*文档版本：v1.0 · 对应 `build.py`（含 §6 已知问题）*