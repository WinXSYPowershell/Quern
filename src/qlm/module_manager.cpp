//   核心业务逻辑  

bool listWebModules() {
    if (!ensureGit()) return false;
    fs::path tempDir = fs::temp_directory_path() / "qlm_weblist_temp";
    if (fs::exists(tempDir)) fs::remove_all(tempDir);
    
    std::cout << COLOR_CYAN << "Fetching module list..." << COLOR_RESET << std::endl;
    if (!runCommand("git clone --depth 1 \"" + REPO_URL + "\" \"" + tempDir.string() + "\"", true)) {
        std::cerr << COLOR_RED << "Failed to clone repository." << COLOR_RESET << std::endl;
        return false;
    }
    
    auto files = searchJsFiles(tempDir, "");
    std::cout << COLOR_BOLD << "\nAvailable Modules (.js):" << COLOR_RESET << std::endl;
    for (const auto& f : files) std::cout << "  - " << f << std::endl;
    if (files.empty()) std::cout << "  (None)" << std::endl;
    
    fs::remove_all(tempDir);
    return true;
}

bool searchWebModules(const std::string& keyword) {
    if (!ensureGit()) return false;
    fs::path tempDir = fs::temp_directory_path() / ("qlm_search_" + keyword);
    if (fs::exists(tempDir)) fs::remove_all(tempDir);

    std::cout << COLOR_CYAN << "Searching remote for '" << keyword << "'..." << COLOR_RESET << std::endl;
    if (!runCommand("git clone --depth 1 \"" + REPO_URL + "\" \"" + tempDir.string() + "\"", true)) {
        std::cerr << COLOR_RED << "Failed to clone repository." << COLOR_RESET << std::endl;
        return false;
    }

    auto matches = searchJsFiles(tempDir, keyword);
    std::cout << COLOR_BOLD << "\nSearch Results:" << COLOR_RESET << std::endl;
    for (const auto& f : matches) std::cout << "  - " << f << std::endl;
    if (matches.empty()) std::cout << "  (No matches)" << std::endl;

    fs::remove_all(tempDir);
    return true;
}

bool listLocalMods() {
    fs::path modsPath = fs::path(MODS_DIR);
    if (!fs::exists(modsPath)) {
        std::cout << COLOR_YELLOW << "Mods folder does not exist." << COLOR_RESET << std::endl;
        return true;
    }
    auto files = searchJsFiles(modsPath, "");
    std::cout << COLOR_BOLD << "\nInstalled Modules:" << COLOR_RESET << std::endl;
    for (const auto& f : files) std::cout << "  - " << f << std::endl;
    if (files.empty()) std::cout << "  (None)" << std::endl;
    return true;
}

bool searchLocalMods(const std::string& keyword) {
    fs::path modsPath = fs::path(MODS_DIR);
    if (!fs::exists(modsPath)) return true;
    auto matches = searchJsFiles(modsPath, keyword);
    std::cout << COLOR_BOLD << "\nLocal Search Results:" << COLOR_RESET << std::endl;
    for (const auto& f : matches) std::cout << "  - " << f << std::endl;
    if (matches.empty()) std::cout << "  (No matches)" << std::endl;
    return true;
}

// 增强版安装：支持版本和哈希校验
bool installModuleWithGit(const std::string& moduleName, const std::string& versionDate = "", const std::string& expectedSha256 = "") {
    if (!ensureGit()) return false;

    fs::path modsDir = fs::path(MODS_DIR);
    if (!fs::exists(modsDir)) {
        try { fs::create_directories(modsDir); } catch (...) { return false; }
    }

    fs::path tempDirPath = fs::current_path() / modsDir / (".qlm_temp_" + moduleName);
    if (fs::exists(tempDirPath)) fs::remove_all(tempDirPath);

    std::cout << COLOR_CYAN << "Downloading module: " << moduleName;
    if (!versionDate.empty()) std::cout << " (Version: " << versionDate << ")";
    std::cout << COLOR_RESET << std::endl;

    // 1. Clone
    std::string cloneCmd = "git clone \"" + REPO_URL + "\" \"" + tempDirPath.string() + "\"";
    if (!runCommand(cloneCmd, true)) {
        std::cerr << COLOR_RED << "Failed to clone repository." << COLOR_RESET << std::endl;
        return false;
    }

    // 2. Checkout specific version if provided
    // 这里的版本日期是用户输入的字符串
    if (!versionDate.empty()) {
        // 将用户输入的逗号分隔日期转换为 git 接受的格式，如果不行，尝试直接使用
        // Git 接受 "YYYY-MM-DD HH:MM:SS" 或相对时间
        std::string formattedDate = versionDate;
        std::replace(formattedDate.begin(), formattedDate.end(), ',', ' '); // 替换逗号为空格
        
        std::string checkoutCmd = "cd \"" + tempDirPath.string() + "\" && git checkout $(git rev-list -n 1 --before=\"" + formattedDate + "\" HEAD)";
        
        
        if (!versionDate.empty()) {
             // 移除 depth 限制的影响，实际上需要完整历史才能按时间回溯
             fs::remove_all(tempDirPath);
             std::string fullCloneCmd = "git clone \"" + REPO_URL + "\" \"" + tempDirPath.string() + "\"";
             if (!runCommand(fullCloneCmd, true)) {
                 std::cerr << COLOR_RED << "Failed to full clone for version check." << COLOR_RESET << std::endl;
                 return false;
             }
             
             // 构建 git log 查询命令找到最接近的 commit
             // 这里简化处理：尝试直接 checkout 日期字符串，git 支持 @{YYYY-MM-DD}
             std::string safeDate = versionDate;
             std::replace(safeDate.begin(), safeDate.end(), '/', '-'); // YYYY/MM/DD -> YYYY-MM-DD
             std::replace(safeDate.begin(), safeDate.end(), ',', ' '); // HH/MM/SS -> HH MM SS (可能需要调整)
             
             // 尝试使用 git checkout "@{2023-10-01 12:00:00}"
             std::string coCmd = "cd \"" + tempDirPath.string() + "\" && git checkout \"@{" + safeDate + "}\"";
             if (!runCommand(coCmd, true)) {
                  std::cerr << COLOR_YELLOW << "[Warning] Could not checkout exact date/version. Using latest available." << COLOR_RESET << std::endl;
             } else {
                  std::cout << COLOR_GREEN << "[Info] Checked out version near: " << versionDate << COLOR_RESET << std::endl;
             }
        }
    }

    // 3. Find File
    fs::path sourceFile = tempDirPath / moduleName;
    std::string realFileName = moduleName;

    if (!fs::exists(sourceFile)) {
        std::string found = findFileCaseInsensitive(tempDirPath, moduleName);
        if (!found.empty()) {
            realFileName = found;
            sourceFile = tempDirPath / found;
        } else {
            std::cerr << COLOR_RED << "Error: File '" << moduleName << "' not found in repository." << COLOR_RESET << std::endl;
            fs::remove_all(tempDirPath);
            return false;
        }
    }

    // 4. Verify SHA256 if provided
    if (!expectedSha256.empty()) {
        std::cout << COLOR_CYAN << "Verifying integrity..." << COLOR_RESET << std::endl;
        std::string actualHash = computeSHA256(sourceFile.string());
        
        // 转换为小写进行比较
        std::string lowerActual = actualHash;
        std::string lowerExpected = expectedSha256;
        std::transform(lowerActual.begin(), lowerActual.end(), lowerActual.begin(), ::tolower);
        std::transform(lowerExpected.begin(), lowerExpected.end(), lowerExpected.begin(), ::tolower);

        if (lowerActual != lowerExpected) {
            std::cerr << COLOR_RED << "[Security Error] SHA256 mismatch!" << COLOR_RESET << std::endl;
            std::cerr << "  Expected: " << expectedSha256 << std::endl;
            std::cerr << "  Actual:   " << actualHash << std::endl;
            std::cerr << "  File may be corrupted or tampered with. Installation aborted." << std::endl;
            fs::remove_all(tempDirPath);
            return false;
        }
        std::cout << COLOR_GREEN << "[Success] Integrity verified." << COLOR_RESET << std::endl;
    }

    // 5. Copy
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

bool deleteModule(const std::string& moduleName) {
    fs::path filePath = fs::path(MODS_DIR) / moduleName;
    if (!fs::exists(filePath)) {
        std::string found = findFileCaseInsensitive(fs::path(MODS_DIR), moduleName);
        if (!found.empty()) filePath = fs::path(MODS_DIR) / found;
        else {
            std::cerr << COLOR_RED << "Error: Module not found: " << moduleName << COLOR_RESET << std::endl;
            return false;
        }
    }
    try {
        fs::remove(filePath);
        std::cout << COLOR_GREEN << "Success: Deleted " << filePath.filename().string() << COLOR_RESET << std::endl;
        return true;
    } catch (...) {
        std::cerr << COLOR_RED << "Error deleting file." << COLOR_RESET << std::endl;
        return false;
    }
}

bool disableModule(const std::string& moduleName) {
    fs::path modsDir = fs::path(MODS_DIR);
    fs::path disabledDir = fs::path(DISABLED_DIR);
    if (!fs::exists(disabledDir)) fs::create_directories(disabledDir);

    fs::path sourcePath = modsDir / moduleName;
    if (!fs::exists(sourcePath)) {
        std::string found = findFileCaseInsensitive(modsDir, moduleName);
        if (!found.empty()) sourcePath = modsDir / found;
        else {
            std::cerr << COLOR_RED << "Error: Module not found in mods." << COLOR_RESET << std::endl;
            return false;
        }
    }

    fs::path destPath = disabledDir / sourcePath.filename();
    try {
        if (fs::exists(destPath)) fs::remove(destPath);
        fs::rename(sourcePath, destPath);
        std::cout << COLOR_GREEN << "Success: Disabled " << sourcePath.filename().string() << COLOR_RESET << std::endl;
        return true;
    } catch (...) {
        std::cerr << COLOR_RED << "Error moving file." << COLOR_RESET << std::endl;
        return false;
    }
}

bool enableModule(const std::string& moduleName) {
    fs::path modsDir = fs::path(MODS_DIR);
    fs::path disabledDir = fs::path(DISABLED_DIR);
    if (!fs::exists(disabledDir)) {
         std::cerr << COLOR_RED << "Error: Disabled folder does not exist." << COLOR_RESET << std::endl;
         return false;
    }

    fs::path sourcePath = disabledDir / moduleName;
    if (!fs::exists(sourcePath)) {
        std::string found = findFileCaseInsensitive(disabledDir, moduleName);
        if (!found.empty()) sourcePath = disabledDir / found;
        else {
            std::cerr << COLOR_RED << "Error: Module not found in disabled." << COLOR_RESET << std::endl;
            return false;
        }
    }

    fs::path destPath = modsDir / sourcePath.filename();
    try {
        if (!fs::exists(modsDir)) fs::create_directories(modsDir);
        if (fs::exists(destPath)) fs::remove(destPath);
        fs::rename(sourcePath, destPath);
        std::cout << COLOR_GREEN << "Success: Enabled " << sourcePath.filename().string() << COLOR_RESET << std::endl;
        return true;
    } catch (...) {
        std::cerr << COLOR_RED << "Error moving file." << COLOR_RESET << std::endl;
        return false;
    }
}