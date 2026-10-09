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
#include <cstdio>
#include <iomanip>
#include <cstdint>
#include <cstring>

// 编译命令：cl /MT /UTF-8 /O2 /std:c++17 /EHsc main.cpp /link shell32.lib

#ifdef _WIN32
    #include <windows.h>
    #include <shlwapi.h> 
    #pragma comment(lib, "Shlwapi.lib")
#else
    #include <unistd.h>
#endif

namespace fs = std::filesystem;

//   配置常量  
const std::string REPO_URL = "https://gitcode.com/2401_86702825/QuernModules-Fork.git";
const std::string MODS_DIR = "mods";
const std::string DISABLED_DIR = "disabled";

//   终端颜色  
const std::string COLOR_RESET = "\033[0m";
const std::string COLOR_YELLOW = "\033[33m";
const std::string COLOR_RED = "\033[31m";
const std::string COLOR_GREEN = "\033[32m";
const std::string COLOR_CYAN = "\033[36m";
const std::string COLOR_BOLD = "\033[1m";

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

// include else code files
#include "utills.cpp"
#include "git.cpp"
#include "module_manager.cpp"
#include "project_manager.cpp"


int main(int argc, char* argv[]) {
    if (argc < 2) {
        printHelp();
        return 1;
    }

    std::string action = argv[1];
    int result = 1;

    if (action == "--Help") { printHelp(); return 0; }
    if (action == "--WebList") return listWebModules() ? 0 : 1;
    if (action == "--ModsList") return listLocalMods() ? 0 : 1;
    if (action == "--InstallPackage") return installPackages() ? 0 : 1;
    if (action == "--ProjectRun") return runProject() ? 0 : 1;

    if (argc < 3 && action != "--NewProject") { 
        std::cerr << COLOR_RED << "Error: Missing argument for " << action << COLOR_RESET << std::endl;
        return 1;
    }

    std::string arg = (argc >= 3) ? argv[2] : "";

    if (action == "--Install") result = installModuleWithGit(arg) ? 0 : 1;
    else if (action == "--Delete") result = deleteModule(arg) ? 0 : 1;
    else if (action == "--Disable") result = disableModule(arg) ? 0 : 1;
    else if (action == "--Enable") result = enableModule(arg) ? 0 : 1;
    else if (action == "--WebSearch") result = searchWebModules(arg) ? 0 : 1;
    else if (action == "--ModsSearch") result = searchLocalMods(arg) ? 0 : 1;
    else if (action == "--NewProject") {
        std::string nameArg = "", verArg = "";
        if (argc >= 3) {
            std::string arg2 = argv[2];
            size_t commaPos = arg2.find(',');
            if (commaPos != std::string::npos) {
                nameArg = arg2.substr(0, commaPos);
                verArg = arg2.substr(commaPos + 1);
            } else {
                nameArg = arg2;
                if (argc >= 4) verArg = argv[3];
            }
        }
        result = createNewProject(nameArg, verArg) ? 0 : 1;
    } else {
        std::cerr << "Unknown action: " << action << std::endl;
        printHelp();
    }

    return result;
}
