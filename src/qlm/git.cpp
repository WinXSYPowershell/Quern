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

bool isCommandAvailable(const std::string& cmd) {
#ifdef _WIN32
    std::string checkCmd = "where " + cmd + " >nul 2>&1";
#else
    std::string checkCmd = "which " + cmd + " > /dev/null 2>&1";
#endif
    return system(checkCmd.c_str()) == 0;
}

bool handleMissingGit() {
    std::cerr << COLOR_RED << "[Error] 'git' is not found in your system PATH." << COLOR_RESET << std::endl;
    std::cout << COLOR_YELLOW << "Git is required for remote operations. Do you want to install it? (y/n): " << COLOR_RESET;
    
    std::string input;
    std::getline(std::cin, input);
    if (!input.empty() && input.back() == '\r') input.pop_back();
    std::transform(input.begin(), input.end(), input.begin(), ::tolower);

    if (input == "y" || input == "yes") {
#ifdef _WIN32
        std::cout << COLOR_CYAN << "[Action] Opening Git download page..." << COLOR_RESET << std::endl;
        ShellExecuteA(NULL, "open", "https://git-scm.com/download/win", NULL, NULL, SW_SHOWNORMAL);
#else
        std::cout << COLOR_CYAN << "[Action] Attempting to install git via apt..." << COLOR_RESET << std::endl;
        if (runCommand("sudo apt update && sudo apt install -y git", false)) {
            if (isCommandAvailable("git")) return true;
        }
#endif
    }
    return false;
}

bool ensureGit() {
    if (isCommandAvailable("git")) return true;

#ifdef _WIN32
    const char* usernameEnv = getenv("USERNAME");
    std::string username = usernameEnv ? usernameEnv : "";
    std::vector<std::string> possiblePaths = {
        "C:\\Program Files\\Git\\cmd",
        "C:\\Program Files (x86)\\Git\\cmd"
    };
    if (!username.empty()) possiblePaths.push_back("C:\\Users\\" + username + "\\AppData\\Local\\Programs\\Git\\cmd");

    for (const auto& path : possiblePaths) {
        if (fs::exists(fs::path(path) / "git.exe")) {
            char* pathEnv = nullptr; size_t len;
            _dupenv_s(&pathEnv, &len, "PATH");
            std::string currentPath(pathEnv ? pathEnv : "");
            free(pathEnv);
            SetEnvironmentVariableA("PATH", (currentPath + ";" + path).c_str());
            if (isCommandAvailable("git")) return true;
        }
    }
#endif
    return handleMissingGit();
}

std::string findFileCaseInsensitive(const fs::path& dir, const std::string& filename) {
    if (!fs::exists(dir)) return "";
    std::string lowerTarget = filename;
    std::transform(lowerTarget.begin(), lowerTarget.end(), lowerTarget.begin(), ::tolower);
    for (const auto& entry : fs::directory_iterator(dir)) {
        if (entry.is_regular_file()) {
            std::string entryName = entry.path().filename().string();
            std::string lowerEntry = entryName;
            std::transform(lowerEntry.begin(), lowerEntry.end(), lowerEntry.begin(), ::tolower);
            if (lowerEntry == lowerTarget) return entryName;
        }
    }
    return "";
}