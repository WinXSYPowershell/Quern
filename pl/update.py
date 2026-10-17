import os
import sys
import subprocess
import tempfile
import shutil
import hashlib
import time
import stat

def run_git_command(args, cwd=None, capture_output=True):
    """执行 Git 命令的辅助函数"""
    cmd = ['git'] + args
    try:
        result = subprocess.run(
            cmd,
            cwd=cwd,
            stdout=subprocess.PIPE if capture_output else None,
            stderr=subprocess.PIPE if capture_output else None,
            text=True,
            check=False
        )
        if capture_output:
            return result.stdout.strip(), result.returncode
        return None, result.returncode
    except Exception as e:
        print(f"执行命令 {cmd} 时出错: {e}")
        return "", 1

def get_all_files_recursive(root_dir):
    """
    使用 os.walk 递归获取目录下所有文件路径（相对于 root_dir）。
    自动忽略 .git, .gitcode 等隐藏版本控制文件夹。
    """
    file_list = set()
    ignore_dirs = {'.git', '.gitcode', '.svn', '.hg'}
    
    for dirpath, dirnames, filenames in os.walk(root_dir):
        # 原地修改 dirnames，阻止 os.walk 进入被忽略的目录
        dirnames[:] = [d for d in dirnames if d not in ignore_dirs]
        
        for filename in filenames:
            full_path = os.path.join(dirpath, filename)
            rel_path = os.path.relpath(full_path, root_dir)
            # 统一使用正斜杠，确保跨平台比较一致
            rel_path = rel_path.replace(os.sep, '/')
            file_list.add(rel_path)
            
    return file_list

def get_sha256(filepath):
    """计算文件的 SHA256 哈希值"""
    sha256_hash = hashlib.sha256()
    try:
        with open(filepath, "rb") as f:
            for byte_block in iter(lambda: f.read(4096), b""):
                sha256_hash.update(byte_block)
        return sha256_hash.hexdigest()
    except Exception as e:
        print(f"警告: 无法读取文件 {filepath}: {e}")
        return ""
def force_remove_directory(path):
    """
    在 Windows 上更可靠地删除目录。
    处理只读属性、文件占用和长路径问题。
    """
    if not os.path.exists(path):
        return True
    
    print(f"正在尝试清理临时目录: {path}")
    
    # 策略 1: 尝试标准的 Python 删除，但先移除只读属性
    def remove_readonly(func, path, excinfo):
        """回调函数，用于处理 PermissionError"""
        print(f"  [调试] 遇到权限错误，尝试移除只读属性: {path}")
        try:
            os.chmod(path, stat.S_IWRITE | stat.S_IREAD)
            func(path)
        except Exception as e:
            print(f"  [警告] 无法移除只读属性或文件: {e}")

    try:
        # 第一次尝试：标准删除，带回调处理只读
        shutil.rmtree(path, onerror=remove_readonly)
        print("  临时目录已自动清理。")
        return True
    except Exception as e:
        print(f"  Python 删除失败: {e}")
        print("  正在尝试使用 CMD 命令强制删除...")

    # 策略 2: 如果 Python 失败，使用 subprocess 调用 CMD 的 rmdir /s /q
    # /s 删除目录树，/q 安静模式不确认
    try:
        # 注意：CMD 的路径如果包含空格需要引号，这里 temp_dir 通常不含空格，但为了安全加上引号
        cmd_str = f'rmdir /s /q "{path}"'
        subprocess.run(cmd_str, shell=True, check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        print("  临时目录已通过 CMD 强制清理。")
        return True
    except Exception as e:
        print(f"  [严重] CMD 强制删除也失败了: {e}")
        return False

def main():
    local_path = os.getcwd()
    
    # 1. 获取远程仓库地址
    remote_url, _ = run_git_command(['config', '--get', 'remote.origin.url'], cwd=local_path)
    if not remote_url:
        remote_url = input("未检测到远程仓库地址，请手动输入远程 URL: ").strip()
        if not remote_url:
            print("错误: 未提供远程仓库地址，退出。")
            sys.exit(1)

    # 创建临时目录
    temp_dir = tempfile.mkdtemp(prefix="git_remote_temp_")
    print(f"临时目录已创建: {temp_dir}")

    try:
        # 2. 克隆远程仓库到临时目录
        print("\n1. 正在克隆远程仓库 (分支: main) 到临时目录...")
        _, clone_code = run_git_command(
            ['clone', '--branch', 'main', '--depth', '1', remote_url, temp_dir],
            capture_output=False
        )
        if clone_code != 0:
            print("错误: 克隆失败，请检查远程地址和网络。")
            sys.exit(1)
        print("克隆完成。")

        # 3. 获取本地和远程的物理文件列表 (使用 os.walk 替代 git ls-files)
        print("\n2. 正在扫描物理文件列表...")
        
        local_files = get_all_files_recursive(local_path)
        remote_files = get_all_files_recursive(temp_dir)

        # 分类文件
        only_in_remote = remote_files - local_files   # 缺失文件 (远程有，本地无)
        only_in_local = local_files - remote_files    # 新增文件 (本地有，远程无)
        in_both = local_files & remote_files          # 共有文件

        print(f"统计: 缺失文件 {len(only_in_remote)} 个, 新增文件 {len(only_in_local)} 个, 共有文件 {len(in_both)} 个。")

        # 4. 处理缺失文件 (远程有，本地无)
        if only_in_remote:
            print("\n3. 处理缺失文件...")
            for file in sorted(only_in_remote):
                print(f"\n发现远程存在但本地缺失的文件: {file}")
                choice = input("是否直接复制进本地？(y/n): ").strip().lower()
                if choice == 'y':
                    src_path = os.path.join(temp_dir, file)
                    dest_path = os.path.join(local_path, file)
                    os.makedirs(os.path.dirname(dest_path), exist_ok=True)
                    shutil.copy2(src_path, dest_path)
                    print(f"已复制: {file}")
                    
                    # 复制后视为新增，询问是否提交
                    commit_choice = input("是否提交此文件更改？(y/n): ").strip().lower()
                    if commit_choice == 'y':
                        msg = input("请输入该文件的提交信息: ").strip()
                        if msg:
                            run_git_command(['add', '--', file], cwd=local_path, capture_output=False)
                            run_git_command(['commit', '-m', msg], cwd=local_path, capture_output=False)
                            print(f"已提交: {file}")
                        else:
                            print("提交信息为空，跳过提交。")
                else:
                    print("已跳过。")
        else:
            print("\n3. 无缺失文件。")

        # 5. 处理共有文件 (使用 SHA256 对比)
        if in_both:
            print("\n4. 对比共有文件内容...")
            for file in sorted(in_both):
                local_file_path = os.path.join(local_path, file)
                remote_file_path = os.path.join(temp_dir, file)
                
                # 计算哈希
                hash_local = get_sha256(local_file_path)
                hash_remote = get_sha256(remote_file_path)
                
                # 如果任一哈希为空（读取错误），跳过对比
                if not hash_local or not hash_remote:
                    continue

                is_same = (hash_local == hash_remote)
                result_str = "相同" if is_same else "不同"
                
                # 仅当不同时才输出详细哈希，避免刷屏，或者您可以选择始终输出
                if not is_same:
                    print(f"文件: {file} | 本地哈希: {hash_local[:8]}... | 远程哈希: {hash_remote[:8]}... | 结果: {result_str}")
                    commit_choice = input("文件内容已更改。是否提交此文件更改？(y/n): ").strip().lower()
                    if commit_choice == 'y':
                        msg = input("请输入该文件的提交信息: ").strip()
                        if msg:
                            run_git_command(['add', '--', file], cwd=local_path, capture_output=False)
                            run_git_command(['commit', '-m', msg], cwd=local_path, capture_output=False)
                            print(f"已提交: {file}")
                        else:
                            print("提交信息为空，跳过提交。")
                else:
                    # 可选：打印相同的文件以确认进度
                    # print(f"文件: {file} | 结果: {result_str}")
                    pass
        else:
            print("\n4. 无共有文件需要对比。")

        # 6. 处理新增文件 (本地有，远程无)
        if only_in_local:
            print("\n5. 处理新增文件...")
            for file in sorted(only_in_local):
                print(f"\n发现本地新增文件: {file}")
                commit_choice = input("是否提交此文件更改？(y/n): ").strip().lower()
                if commit_choice == 'y':
                    msg = input("请输入该文件的提交信息: ").strip()
                    if msg:
                        run_git_command(['add', '--', file], cwd=local_path, capture_output=False)
                        run_git_command(['commit', '-m', msg], cwd=local_path, capture_output=False)
                        print(f"已提交: {file}")
                    else:
                        print("提交信息为空，跳过提交。")
                else:
                    print("已跳过。")
        else:
            print("\n5. 无新增文件。")

        # 7. 统一推送到远程
        print("\n6. 正在推送更改到 origin main ...")
        _, push_code = run_git_command(['push', 'origin', 'main'], cwd=local_path, capture_output=False)
        if push_code == 0:
            print("推送成功！所有操作已完成。")
        else:
            print("推送失败，请检查网络或远程仓库冲突。")

    finally:
        # 8. 清理临时目录 (使用增强版清理函数)
        print("\n7. 正在清理临时目录...")
        if os.path.exists(temp_dir):
            # 增加一个小延迟，确保 Git 子进程完全退出
            time.sleep(1) 
            success = force_remove_directory(temp_dir)
            if not success and os.path.exists(temp_dir):
                print(f"请手动删除: {temp_dir}")
        
        # 9. 退出
        print("程序退出。")
        sys.exit(0)

if __name__ == "__main__":
    main()
