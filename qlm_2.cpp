#include <iostream>
#include <string>
#include <filesystem>
#include <cstdlib>
#include <algorithm>
#include <vector>
#include <fstream>
#include <sstream>
#include <map>
#include <regex>
#include <cctype>
#include <cstdio> // For popen/pclose

#ifdef _WIN32
    #include <windows.h>
    #include <shlwapi.h> 
    #pragma comment(lib, "Shlwapi.lib")
#else
    #include <unistd.h>
#endif

namespace fs = std::filesystem;

// 配置常量
const std::string REPO_URL = "https://gitcode.com/2401_86702825/QuernModules-Fork.git";
const std::string MODS_DIR = "mods";
const std::string DISABLED_DIR = "disabled";

// 终端颜色代码
const std::string COLOR_RESET = "\033[0m";
const std::string COLOR_YELLOW = "\033[33m";
const std::string COLOR_RED = "\033[31m";
const std::string COLOR_GREEN = "\033[32m";
const std::string COLOR_CYAN = "\033[36m";
const std::string COLOR_BOLD = "\033[1m";

void printHelp() {
    std::cout << COLOR_BOLD << "Quern Libraries Manager (QLM)" << COLOR_RESET << std::endl;
    std::cout << "Usage: qlm [options]" << std::endl;
    std::cout << std::endl;
    std::cout << "Options:" << std::endl;
    std::cout << "  --install <module_name.js>     Download and install a module from GitCode." << std::endl;
    std::cout << "  --delete <module_name.js>      Delete a module from the mods folder." << std::endl;
    std::cout << "  --disable <module_name.js>     Move a module to the 'disabled' folder." << std::endl;
    std::cout << "  --enable <module_name.js>      Move a module from 'disabled' back to 'mods'." << std::endl;
    std::cout << "  --weblist                      List all available .js files in the remote repository." << std::endl;
    std::cout << "  --modslist                     List all installed .js files in the local mods folder." << std::endl;
    std::cout << "  --websearch <keyword>          Search for modules in the remote repository by filename." << std::endl;
    std::cout << "  --modssearch <keyword>         Search for modules in the local mods folder by filename." << std::endl;
    std::cout << "  --NewProject <name>,<ver>      Create a new project with src/main.q and config.toml." << std::endl;
    std::cout << "  --InstallPackage               Read local .toml and install required packages." << std::endl;
    std::cout << "  --ProjectRun                   Check dependencies, install missing ones, and run the project." << std::endl;
    std::cout << "  --help                         Show this help message." << std::endl;
}

// 辅助函数：执行系统命令
bool runCommand(const std::string& cmd, bool silent = false) {
    std::string finalCmd = cmd;
    if (silent) {
#ifdef _WIN32
        finalCmd += " >nul 2>&1";
#else
        finalCmd += " > /dev/null 2>&1";
#endif
    }
    int result = system(finalCmd.c_str());
    return result == 0;
}

// 检查命令是否可用
bool isCommandAvailable(const std::string& cmd) {
#ifdef _WIN32
    std::string checkCmd = "where " + cmd + " >nul 2>&1";
#else
    std::string checkCmd = "which " + cmd + " > /dev/null 2>&1";
#endif
    return system(checkCmd.c_str()) == 0;
}

// 处理 Git 未安装的情况
bool handleMissingGit() {
    std::cerr << COLOR_RED << "[Error] 'git' is not found in your system PATH." << COLOR_RESET << std::endl;
    std::cout << COLOR_YELLOW << "Git is required for remote operations. Do you want to install it? (y/n): " << COLOR_RESET;
    
    std::string input;
    std::getline(std::cin, input);
    
    // 清理输入
    if (!input.empty() && input.back() == '\r') input.pop_back();
    std::transform(input.begin(), input.end(), input.begin(), ::tolower);

    if (input == "y" || input == "yes") {
#ifdef _WIN32
        std::cout << COLOR_CYAN << "[Action] Opening Git download page in browser..." << COLOR_RESET << std::endl;
        ShellExecuteA(NULL, "open", "https://git-scm.com/download/win", NULL, NULL, SW_SHOWNORMAL);
        std::cout << "Please install Git, ensure 'Add to PATH' is selected, and restart this tool." << std::endl;
#else
        std::cout << COLOR_CYAN << "[Action] Attempting to install git via apt (requires sudo)..." << COLOR_RESET << std::endl;
        std::string installCmd = "sudo apt update && sudo apt install -y git";
        if (runCommand(installCmd, false)) {
            std::cout << COLOR_GREEN << "[Success] Git installation command executed. Please verify." << COLOR_RESET << std::endl;
            if (isCommandAvailable("git")) return true;
        } else {
            std::cerr << COLOR_RED << "[Failed] Could not automatically install git. Please install it manually." << COLOR_RESET << std::endl;
        }
#endif
    } else {
        std::cout << COLOR_RED << "[Exit] Operation cancelled. Git is required." << COLOR_RESET << std::endl;
    }
    
    return false;
}

// 确保 Git 可用
bool ensureGit() {
    if (isCommandAvailable("git")) {
        return true;
    }

#ifdef _WIN32
    std::cout << COLOR_YELLOW << "[Info] 'git' not found in PATH. Searching common installation directories..." << COLOR_RESET << std::endl;
    
    const char* usernameEnv = getenv("USERNAME");
    std::string username = usernameEnv ? usernameEnv : "";
    
    std::vector<std::string> possiblePaths;
    possiblePaths.push_back("C:\\Program Files\\Git\\cmd");
    possiblePaths.push_back("C:\\Program Files (x86)\\Git\\cmd");
    if (!username.empty()) {
        possiblePaths.push_back(std::string("C:\\Users\\") + username + "\\AppData\\Local\\Programs\\Git\\cmd");
    }

    for (const auto& path : possiblePaths) {
        fs::path gitExe = fs::path(path) / "git.exe";
        if (fs::exists(gitExe)) {
            std::cout << COLOR_GREEN << "[Found] Git located at: " << path << COLOR_RESET << std::endl;
            std::cout << "[Info] Adding to current session PATH..." << std::endl;
            
            char* pathEnv = nullptr;
            size_t len;
            _dupenv_s(&pathEnv, &len, "PATH");
            std::string currentPath(pathEnv ? pathEnv : "");
            free(pathEnv);

            std::string newPath = currentPath + ";" + path;
            SetEnvironmentVariableA("PATH", newPath.c_str());
            
            if (isCommandAvailable("git")) {
                std::cout << COLOR_GREEN << "[Success] Git is now accessible." << COLOR_RESET << std::endl;
                return true;
            }
        }
    }
#endif

    return handleMissingGit();
}

// 大小写不敏感查找文件
std::string findFileCaseInsensitive(const fs::path& dir, const std::string& filename) {
    if (!fs::exists(dir)) return "";

    std::string lowerTarget = filename;
    std::transform(lowerTarget.begin(), lowerTarget.end(), lowerTarget.begin(), ::tolower);

    for (const auto& entry : fs::directory_iterator(dir)) {
        if (entry.is_regular_file()) {
            std::string entryName = entry.path().filename().string();
            std::string lowerEntry = entryName;
            std::transform(lowerEntry.begin(), lowerEntry.end(), lowerEntry.begin(), ::tolower);
            
            if (lowerEntry == lowerTarget) {
                return entryName;
            }
        }
    }
    return "";
}

// 递归查找目录下所有匹配关键字的 .js 文件
std::vector<std::string> searchJsFiles(const fs::path& dir, const std::string& keyword) {
    std::vector<std::string> matches;
    if (!fs::exists(dir)) return matches;

    std::string lowerKeyword = keyword;
    std::transform(lowerKeyword.begin(), lowerKeyword.end(), lowerKeyword.begin(), ::tolower);

    for (const auto& entry : fs::recursive_directory_iterator(dir)) {
        if (entry.is_regular_file() && entry.path().extension() == ".js") {
            std::string fileName = entry.path().filename().string();
            std::string lowerFileName = fileName;
            std::transform(lowerFileName.begin(), lowerFileName.end(), lowerFileName.begin(), ::tolower);
            
            if (lowerFileName.find(lowerKeyword) != std::string::npos) {
                matches.push_back(fs::relative(entry.path(), dir).string());
            }
        }
    }
    std::sort(matches.begin(), matches.end());
    return matches;
}

// 功能：--weblist
bool listWebModules() {
    if (!ensureGit()) return false;

    fs::path tempDir = fs::temp_directory_path() / "qlm_weblist_temp";
    if (fs::exists(tempDir)) {
        fs::remove_all(tempDir);
    }

    std::cout << COLOR_CYAN << "Fetching module list from repository..." << COLOR_RESET << std::endl;
    
    std::string cloneCmd = "git clone --depth 1 \"" + REPO_URL + "\" \"" + tempDir.string() + "\"";
    if (!runCommand(cloneCmd, true)) {
        std::cerr << COLOR_RED << "Failed to clone repository for listing." << COLOR_RESET << std::endl;
        return false;
    }

    std::vector<std::string> files = searchJsFiles(tempDir, ""); 
    
    std::cout << COLOR_BOLD << "\nAvailable Modules (.js):" << COLOR_RESET << std::endl;
    if (files.empty()) {
        std::cout << "  (No .js files found)" << std::endl;
    } else {
        for (const auto& f : files) {
            std::cout << "  - " << f << std::endl;
        }
    }
    std::cout << std::endl;

    fs::remove_all(tempDir);
    return true;
}

// 功能：--websearch
bool searchWebModules(const std::string& keyword) {
    if (!ensureGit()) return false;

    fs::path tempDir = fs::temp_directory_path() / ("qlm_search_temp_" + keyword);
    if (fs::exists(tempDir)) {
        fs::remove_all(tempDir);
    }

    std::cout << COLOR_CYAN << "Searching for '" << keyword << "' in remote repository..." << COLOR_RESET << std::endl;
    
    std::string cloneCmd = "git clone --depth 1 \"" + REPO_URL + "\" \"" + tempDir.string() + "\"";
    if (!runCommand(cloneCmd, true)) {
        std::cerr << COLOR_RED << "Failed to clone repository for searching." << COLOR_RESET << std::endl;
        return false;
    }

    std::vector<std::string> matches = searchJsFiles(tempDir, keyword);
    
    std::cout << COLOR_BOLD << "\nSearch Results for '" << keyword << "':" << COLOR_RESET << std::endl;
    if (matches.empty()) {
        std::cout << "  (No matching files found)" << std::endl;
    } else {
        for (const auto& f : matches) {
            std::cout << "  - " << f << std::endl;
        }
    }
    std::cout << std::endl;

    fs::remove_all(tempDir);
    return true;
}

// 功能：--modslist
bool listLocalMods() {
    fs::path modsPath = fs::path(MODS_DIR);
    if (!fs::exists(modsPath)) {
        std::cout << COLOR_YELLOW << "Mods folder '" << MODS_DIR << "' does not exist yet." << COLOR_RESET << std::endl;
        return true;
    }

    std::vector<std::string> files = searchJsFiles(modsPath, "");
    
    std::cout << COLOR_BOLD << "\nInstalled Modules (" << MODS_DIR << "):" << COLOR_RESET << std::endl;
    if (files.empty()) {
        std::cout << "  (No .js files found)" << std::endl;
    } else {
        for (const auto& f : files) {
            std::cout << "  - " << f << std::endl;
        }
    }
    std::cout << std::endl;
    return true;
}

// 功能：--modssearch
bool searchLocalMods(const std::string& keyword) {
    fs::path modsPath = fs::path(MODS_DIR);
    if (!fs::exists(modsPath)) {
        std::cout << COLOR_YELLOW << "Mods folder '" << MODS_DIR << "' does not exist yet." << COLOR_RESET << std::endl;
        return true;
    }

    std::vector<std::string> matches = searchJsFiles(modsPath, keyword);
    
    std::cout << COLOR_BOLD << "\nLocal Search Results for '" << keyword << "':" << COLOR_RESET << std::endl;
    if (matches.empty()) {
        std::cout << "  (No matching files found)" << std::endl;
    } else {
        for (const auto& f : matches) {
            std::cout << "  - " << f << std::endl;
        }
    }
    std::cout << std::endl;
    return true;
}

// 功能：--install
bool installModuleWithGit(const std::string& moduleName) {
    if (!ensureGit()) {
        return false;
    }

    fs::path modsDir = fs::path(MODS_DIR);
    if (!fs::exists(modsDir)) {
        try { fs::create_directories(modsDir); } catch (...) { return false; }
    }

    fs::path currentPath = fs::current_path();
    fs::path tempDirPath = currentPath / modsDir / (".qlm_temp_" + moduleName);
    
    if (fs::exists(tempDirPath)) {
        fs::remove_all(tempDirPath);
    }
    
    std::cout << COLOR_CYAN << "Downloading module from: " << REPO_URL << COLOR_RESET << std::endl;

    std::string cloneCmd = "git clone --depth 1 \"" + REPO_URL + "\" \"" + tempDirPath.string() + "\"";
    
    std::cout << "Cloning repository (shallow)..." << std::endl;
    if (!runCommand(cloneCmd, false)) {
        std::cerr << COLOR_RED << "Failed to clone repository." << COLOR_RESET << std::endl;
        fs::remove_all(tempDirPath);
        return false;
    }

    fs::path sourceFile = tempDirPath / moduleName;
    std::string realFileName = moduleName;

    if (!fs::exists(sourceFile)) {
        std::string found = findFileCaseInsensitive(tempDirPath, moduleName);
        if (!found.empty()) {
            std::cout << COLOR_YELLOW << "[Warning] File name case mismatch. Found '" << found << "'." << COLOR_RESET << std::endl;
            realFileName = found;
            sourceFile = tempDirPath / found;
        } else {
            std::cerr << COLOR_RED << "Error: File '" << moduleName << "' not found in repository root." << COLOR_RESET << std::endl;
            fs::remove_all(tempDirPath);
            return false;
        }
    }

    fs::path destFile = modsDir / realFileName;
    try {
        fs::copy_file(sourceFile, destFile, fs::copy_options::overwrite_existing);
        std::cout << COLOR_GREEN << "Success: Module installed to " << destFile.string() << COLOR_RESET << std::endl;
    } catch (const fs::filesystem_error& e) {
        std::cerr << COLOR_RED << "Error copying: " << e.what() << COLOR_RESET << std::endl;
        fs::remove_all(tempDirPath);
        return false;
    }

    fs::remove_all(tempDirPath);
    return true;
}

// 功能：--delete
bool deleteModule(const std::string& moduleName) {
    fs::path filePath = fs::path(MODS_DIR) / moduleName;
    if (!fs::exists(filePath)) {
        std::string found = findFileCaseInsensitive(fs::path(MODS_DIR), moduleName);
        if (!found.empty()) {
            filePath = fs::path(MODS_DIR) / found;
        } else {
            std::cerr << COLOR_RED << "Error: Module not found in '" << MODS_DIR << "': " << moduleName << COLOR_RESET << std::endl;
            return false;
        }
    }
    try {
        fs::remove(filePath);
        std::cout << COLOR_GREEN << "Success: Module deleted: " << filePath.filename().string() << COLOR_RESET << std::endl;
        return true;
    } catch (const fs::filesystem_error& e) {
        std::cerr << COLOR_RED << "Error deleting: " << e.what() << COLOR_RESET << std::endl;
        return false;
    }
}

// 功能：--disable (Move to disabled folder)
bool disableModule(const std::string& moduleName) {
    fs::path modsDir = fs::path(MODS_DIR);
    fs::path disabledDir = fs::path(DISABLED_DIR);

    if (!fs::exists(disabledDir)) {
        try { fs::create_directories(disabledDir); } catch (...) { 
            std::cerr << COLOR_RED << "Error creating disabled directory." << COLOR_RESET << std::endl;
            return false;
        }
    }

    fs::path sourcePath = modsDir / moduleName;
    if (!fs::exists(sourcePath)) {
        std::string found = findFileCaseInsensitive(modsDir, moduleName);
        if (!found.empty()) {
            sourcePath = modsDir / found;
        } else {
            std::cerr << COLOR_RED << "Error: Module not found in '" << MODS_DIR << "': " << moduleName << COLOR_RESET << std::endl;
            return false;
        }
    }

    fs::path destPath = disabledDir / sourcePath.filename();
    
    try {
        if (fs::exists(destPath)) {
            fs::remove(destPath);
        }
        fs::rename(sourcePath, destPath);
        std::cout << COLOR_GREEN << "Success: Module disabled (moved to " << DISABLED_DIR << "/" << sourcePath.filename().string() << ")" << COLOR_RESET << std::endl;
        return true;
    } catch (const fs::filesystem_error& e) {
        std::cerr << COLOR_RED << "Error moving file: " << e.what() << COLOR_RESET << std::endl;
        return false;
    }
}

// 功能：--enable (Move from disabled to mods)
bool enableModule(const std::string& moduleName) {
    fs::path modsDir = fs::path(MODS_DIR);
    fs::path disabledDir = fs::path(DISABLED_DIR);

    if (!fs::exists(disabledDir)) {
         std::cerr << COLOR_RED << "Error: '" << DISABLED_DIR << "' folder does not exist." << COLOR_RESET << std::endl;
         return false;
    }

    fs::path sourcePath = disabledDir / moduleName;
    if (!fs::exists(sourcePath)) {
        std::string found = findFileCaseInsensitive(disabledDir, moduleName);
        if (!found.empty()) {
            sourcePath = disabledDir / found;
        } else {
            std::cerr << COLOR_RED << "Error: Module not found in '" << DISABLED_DIR << "': " << moduleName << COLOR_RESET << std::endl;
            return false;
        }
    }

    fs::path destPath = modsDir / sourcePath.filename();

    try {
        if (!fs::exists(modsDir)) {
            fs::create_directories(modsDir);
        }
        if (fs::exists(destPath)) {
            fs::remove(destPath);
        }
        fs::rename(sourcePath, destPath);
        std::cout << COLOR_GREEN << "Success: Module enabled (moved to " << MODS_DIR << "/" << sourcePath.filename().string() << ")" << COLOR_RESET << std::endl;
        return true;
    } catch (const fs::filesystem_error& e) {
        std::cerr << COLOR_RED << "Error moving file: " << e.what() << COLOR_RESET << std::endl;
        return false;
    }
}

// 功能：--NewProject
bool createNewProject(const std::string& nameArg, const std::string& verArg) {
    std::string projectName = nameArg;
    std::string version = verArg;

    // 清理可能存在的首尾引号
    auto cleanStr = [](std::string& s) {
        if (!s.empty() && (s.front() == '"' || s.front() == '\'')) s.erase(0, 1);
        if (!s.empty() && (s.back() == '"' || s.back() == '\'')) s.pop_back();
    };
    cleanStr(projectName);
    cleanStr(version);

    if (projectName.empty() || version.empty()) {
        std::cerr << COLOR_RED << "Error: Invalid project name or version." << COLOR_RESET << std::endl;
        return false;
    }

    fs::path projDir = fs::path(projectName);
    if (fs::exists(projDir)) {
        std::cerr << COLOR_RED << "Error: Directory '" << projectName << "' already exists." << COLOR_RESET << std::endl;
        return false;
    }

    try {
        fs::create_directories(projDir / "src");
        
        // 创建 main.q
        std::ofstream mainQ(projDir / "src" / "main.q");
        if (!mainQ) throw std::runtime_error("Cannot create main.q");
        mainQ << "Function \"Main\"() {\n    Console.Info(\"Hello World!\");\n}\n";
        mainQ.close();

        // 创建 .toml
        std::string tomlName = projectName + ".toml";
        std::ofstream tomlFile(projDir / tomlName);
        if (!tomlFile) throw std::runtime_error("Cannot create .toml");
        tomlFile << "[edition]\n";
        tomlFile << "name = " << projectName << "\n";
        tomlFile << "ver = " << version << "\n\n";
        tomlFile << "[packages]\n";
        tomlFile << "# EmptyNow. Format: package_name , need = {package1, package2...}\n";
        tomlFile.close();

        std::cout << COLOR_GREEN << "Success: Project '" << projectName << "' created." << COLOR_RESET << std::endl;
        return true;
    } catch (const std::exception& e) {
        std::cerr << COLOR_RED << "Error creating project: " << e.what() << COLOR_RESET << std::endl;
        return false;
    }
}

// 功能：--InstallPackage
bool installPackages() {
    std::string tomlFile = "";
    for (const auto& entry : fs::directory_iterator(".")) {
        if (entry.is_regular_file() && entry.path().extension() == ".toml") {
            tomlFile = entry.path().string();
            break;
        }
    }

    if (tomlFile.empty()) {
        std::cerr << COLOR_RED << "Error: No .toml configuration file found in the current directory." << COLOR_RESET << std::endl;
        return false;
    }

    std::cout << COLOR_CYAN << "Reading configuration from: " << tomlFile << COLOR_RESET << std::endl;

    std::ifstream file(tomlFile);
    if (!file) {
        std::cerr << COLOR_RED << "Error: Cannot open " << tomlFile << COLOR_RESET << std::endl;
        return false;
    }
    std::string content((std::istreambuf_iterator<char>(file)), std::istreambuf_iterator<char>());
    file.close();

    std::vector<std::string> packages;
    // 使用正则表达式匹配 need = { ... }
    std::regex reg(R"(need\s*=\s*\{([^}]*)\})");
    std::smatch match;
    if (std::regex_search(content, match, reg)) {
        std::string pkgStr = match[1].str();
        std::stringstream ss(pkgStr);
        std::string item;
        while (std::getline(ss, item, ',')) {
            // 去除空白字符和引号
            item.erase(std::remove_if(item.begin(), item.end(), [](unsigned char c){ return std::isspace(c); }), item.end());
            item.erase(std::remove(item.begin(), item.end(), '"'), item.end());
            item.erase(std::remove(item.begin(), item.end(), '\''), item.end());
            if (!item.empty()) {
                packages.push_back(item);
            }
        }
    }

    if (packages.empty()) {
        std::cout << COLOR_YELLOW << "No packages found to install in [packges] section." << COLOR_RESET << std::endl;
        return true;
    }

    std::cout << COLOR_BOLD << "Found " << packages.size() << " package(s) to install:" << COLOR_RESET << std::endl;
    for (const auto& pkg : packages) {
        std::cout << "  - " << pkg << std::endl;
    }
    std::cout << std::endl;

    bool allSuccess = true;
    for (const auto& pkg : packages) {
        std::cout << COLOR_CYAN << "Installing package: " << pkg << COLOR_RESET << std::endl;
        if (!installModuleWithGit(pkg)) {
            std::cerr << COLOR_RED << "Failed to install package: " << pkg << COLOR_RESET << std::endl;
            allSuccess = false;
        }
    }

    return allSuccess;
}

// 功能：--ProjectRun
bool runProject() {
    // 1. 查找 .toml 文件
    std::string tomlFile = "";
    std::string projectName = "";
    for (const auto& entry : fs::directory_iterator(".")) {
        if (entry.is_regular_file() && entry.path().extension() == ".toml") {
            tomlFile = entry.path().string();
            projectName = entry.path().stem().string();
            break;
        }
    }

    if (tomlFile.empty()) {
        std::cerr << COLOR_RED << "Error: No .toml configuration file found in the current directory." << COLOR_RESET << std::endl;
        return false;
    }

    std::cout << COLOR_CYAN << "Found project config: " << tomlFile << COLOR_RESET << std::endl;

    // 2. 解析依赖
    std::ifstream file(tomlFile);
    if (!file) {
        std::cerr << COLOR_RED << "Error: Cannot open " << tomlFile << COLOR_RESET << std::endl;
        return false;
    }
    std::string content((std::istreambuf_iterator<char>(file)), std::istreambuf_iterator<char>());
    file.close();

    std::vector<std::string> requiredPackages;
    std::regex reg(R"(need\s*=\s*\{([^}]*)\})");
    std::smatch match;
    if (std::regex_search(content, match, reg)) {
        std::string pkgStr = match[1].str();
        std::stringstream ss(pkgStr);
        std::string item;
        while (std::getline(ss, item, ',')) {
            item.erase(std::remove_if(item.begin(), item.end(), [](unsigned char c){ return std::isspace(c); }), item.end());
            item.erase(std::remove(item.begin(), item.end(), '"'), item.end());
            item.erase(std::remove(item.begin(), item.end(), '\''), item.end());
            if (!item.empty()) {
                requiredPackages.push_back(item);
            }
        }
    }

    // 3. 获取本地已安装的模块
    std::vector<std::string> installedMods;
    fs::path modsPath = fs::path(MODS_DIR);
    if (fs::exists(modsPath)) {
        for (const auto& entry : fs::directory_iterator(modsPath)) {
            if (entry.is_regular_file() && entry.path().extension() == ".js") {
                installedMods.push_back(entry.path().filename().string());
            }
        }
    }

    // 4. 对比并安装缺失依赖
    std::vector<std::string> missingPackages;
    for (const auto& req : requiredPackages) {
        bool found = false;
        // 简单比较文件名（假设包名就是文件名，或者包含在文件名中）
        // 这里为了严谨，我们检查 installedMods 中是否有以 req 开头或完全匹配的文件
        for (const auto& inst : installedMods) {
            std::string lowerReq = req;
            std::string lowerInst = inst;
            std::transform(lowerReq.begin(), lowerReq.end(), lowerReq.begin(), ::tolower);
            std::transform(lowerInst.begin(), lowerInst.end(), lowerInst.begin(), ::tolower);
            
            // 如果安装包名是 "lib.js"，而依赖是 "lib"，则匹配
            if (lowerInst.find(lowerReq) != std::string::npos || lowerReq.find(lowerInst) != std::string::npos) {
                found = true;
                break;
            }
        }
        
        if (!found) {
            missingPackages.push_back(req);
        }
    }

    if (!missingPackages.empty()) {
        std::cout << COLOR_YELLOW << "[Info] Missing dependencies detected:" << COLOR_RESET << std::endl;
        for (const auto& pkg : missingPackages) {
            std::cout << "  - " << pkg << std::endl;
        }
        std::cout << COLOR_CYAN << "Attempting to install missing dependencies..." << COLOR_RESET << std::endl;
        
        bool installSuccess = true;
        for (const auto& pkg : missingPackages) {
            if (!installModuleWithGit(pkg)) {
                std::cerr << COLOR_RED << "Failed to install dependency: " << pkg << COLOR_RESET << std::endl;
                installSuccess = false;
            }
        }
        
        if (!installSuccess) {
            std::cerr << COLOR_RED << "Dependency installation failed. Aborting run." << COLOR_RESET << std::endl;
            return false;
        }
        std::cout << COLOR_GREEN << "[Success] All dependencies are ready." << COLOR_RESET << std::endl;
    } else {
        std::cout << COLOR_GREEN << "[Info] All dependencies are satisfied." << COLOR_RESET << std::endl;
    }

    // 5. 运行项目
    // 检查 Quernc.exe 是否存在
    std::string compilerPath = "Quernc.exe";
    if (!fs::exists(compilerPath)) {
        // 尝试在当前目录查找，如果没找到，提示用户
        std::cerr << COLOR_RED << "Error: 'Quernc.exe' not found in the current directory." << COLOR_RESET << std::endl;
        std::cout << COLOR_YELLOW << "Please ensure Quernc.exe is in the same folder as this tool or in your PATH." << COLOR_RESET << std::endl;
        return false;
    }

    // 检查 src/main.q 是否存在
    fs::path mainScript = fs::path("src") / "main.q";
    if (!fs::exists(mainScript)) {
        std::cerr << COLOR_RED << "Error: 'src/main.q' not found." << COLOR_RESET << std::endl;
        return false;
    }

    std::string runCmd = compilerPath + " --Run src\\main.q";
#ifdef _WIN32
    // Windows 使用反斜杠
#else
    runCmd = compilerPath + " --Run src/main.q";
#endif

    std::cout << COLOR_BOLD << "\n[Running] " << runCmd << COLOR_RESET << std::endl;
    std::cout << "----------------------------------------" << std::endl;

    // 使用 popen 执行命令并捕获输出
#ifdef _WIN32
    FILE* pipe = _popen(runCmd.c_str(), "r");
#else
    FILE* pipe = popen(runCmd.c_str(), "r");
#endif

    if (!pipe) {
        std::cerr << COLOR_RED << "Error: Failed to execute command." << COLOR_RESET << std::endl;
        return false;
    }

    char buffer[256];
    while (fgets(buffer, sizeof(buffer), pipe) != NULL) {
        std::cout << buffer;
        fflush(stdout); // 确保实时输出
    }

#ifdef _WIN32
    int status = _pclose(pipe);
#else
    int status = pclose(pipe);
#endif

    std::cout << "----------------------------------------" << std::endl;
    
    if (status == 0) {
        std::cout << COLOR_GREEN << "[Finished] Project executed successfully." << COLOR_RESET << std::endl;
        return true;
    } else {
        std::cerr << COLOR_RED << "[Error] Project execution failed with code: " << status << COLOR_RESET << std::endl;
        return false;
    }
}

int main(int argc, char* argv[]) {
    if (argc < 2) {
        printHelp();
        return 1;
    }

    std::string action = argv[1];
    int result = 1;

    // 不需要参数的命令
    if (action == "--help") {
        printHelp();
        return 0;
    }
    if (action == "--weblist") {
        return listWebModules() ? 0 : 1;
    }
    if (action == "--modslist") {
        return listLocalMods() ? 0 : 1;
    }
    if (action == "--InstallPackage") {
        return installPackages() ? 0 : 1;
    }
    if (action == "--ProjectRun") {
        return runProject() ? 0 : 1;
    }

    // 需要参数的命令
    if (argc < 3 && action != "--NewProject") { 
        std::cerr << COLOR_RED << "Error: Missing argument for " << action << COLOR_RESET << std::endl;
        printHelp();
        return 1;
    }

    std::string moduleName = (argc >= 3) ? argv[2] : "";

    if (action == "--install") {
        result = installModuleWithGit(moduleName) ? 0 : 1;
    } else if (action == "--delete") {
        result = deleteModule(moduleName) ? 0 : 1;
    } else if (action == "--disable") {
        result = disableModule(moduleName) ? 0 : 1;
    } else if (action == "--enable") {
        result = enableModule(moduleName) ? 0 : 1;
    } else if (action == "--websearch") {
        result = searchWebModules(moduleName) ? 0 : 1;
    } else if (action == "--modssearch") {
        result = searchLocalMods(moduleName) ? 0 : 1;
    } else if (action == "--NewProject") {
        std::string nameArg = "", verArg = "";
        if (argc >= 3) {
            std::string arg2 = argv[2];
            size_t commaPos = arg2.find(',');
            if (commaPos != std::string::npos) {
                // 格式: "NAME","VER" 或 NAME,VER
                nameArg = arg2.substr(0, commaPos);
                verArg = arg2.substr(commaPos + 1);
            } else {
                // 格式: NAME VER
                nameArg = arg2;
                if (argc >= 4) {
                    verArg = argv[3];
                }
            }
        }
        result = createNewProject(nameArg, verArg) ? 0 : 1;
    } else {
        std::cerr << "Unknown action: " << action << std::endl;
        printHelp();
    }

    return result;
}
