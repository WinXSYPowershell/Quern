bool createNewProject(const std::string& nameArg, const std::string& verArg) {
    std::string projectName = nameArg;
    std::string version = verArg;
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
        std::cerr << COLOR_RED << "Error: Directory already exists." << COLOR_RESET << std::endl;
        return false;
    }

    try {
        fs::create_directories(projDir / "src");
        std::ofstream mainQ(projDir / "src" / "main.q");
        mainQ << "Function \"Main\"() {\n    Console.Info(\"Hello World!\");\n}\n";
        mainQ.close();

        std::string tomlName = projectName + ".toml";
        std::ofstream tomlFile(projDir / tomlName);
        tomlFile << "[edition]\n";
        tomlFile << "name = " << projectName << "\n";
        tomlFile << "ver = " << version << "\n\n";
        tomlFile << "[packages]\n";
        tomlFile << "# New Format Example:\n";
        tomlFile << "# need = {\n";
        tomlFile << "#   ModuleName.js ver = 2023/10/01,12/00/00 sha256 = abc123...\n";
        tomlFile << "# }\n";
        tomlFile << "need = {\n\n}\n";
        tomlFile.close();

        std::cout << COLOR_GREEN << "Success: Project '" << projectName << "' created." << COLOR_RESET << std::endl;
        return true;
    } catch (...) {
        std::cerr << COLOR_RED << "Error creating project." << COLOR_RESET << std::endl;
        return false;
    }
}

// 结构体存储包信息
struct PackageInfo {
    std::string name;
    std::string version; // YYYY/MM/DD,HH/MM/SS
    std::string sha256;
    bool hasVersion;
    bool hasSha256;
};

bool installPackages() {
    std::string tomlFile = "";
    for (const auto& entry : fs::directory_iterator(".")) {
        if (entry.is_regular_file() && entry.path().extension() == ".toml") {
            tomlFile = entry.path().string();
            break;
        }
    }

    if (tomlFile.empty()) {
        std::cerr << COLOR_RED << "Error: No .toml file found." << COLOR_RESET << std::endl;
        return false;
    }

    std::cout << COLOR_CYAN << "Reading config: " << tomlFile << COLOR_RESET << std::endl;
    std::ifstream file(tomlFile);
    if (!file) return false;
    std::string content((std::istreambuf_iterator<char>(file)), std::istreambuf_iterator<char>());
    file.close();

    std::vector<PackageInfo> packages;
    
    // 查找 need = { ... }
    std::regex reg(R"(need\s*=\s*\{([^}]*)\})");
    std::smatch match;
    
    if (std::regex_search(content, match, reg)) {
        std::string block = match[1].str();
        std::istringstream iss(block);
        std::string line;
        
        while (std::getline(iss, line)) {
            // 跳过空行或注释
            if (line.empty() || line.find('#') != std::string::npos) continue;
            
            PackageInfo pkg;
            pkg.hasVersion = false;
            pkg.hasSha256 = false;
            
            // 尝试解析新格式: NAME ver = V sha256 = S
            // 或者旧格式: NAME
            // 清理行尾逗号
            if (!line.empty() && line.back() == ',') line.pop_back();
            // 清理空白
            line.erase(0, line.find_first_not_of(" \t"));
            if (line.empty()) continue;

            // 解析逻辑
            // 1. 提取第一个单词作为名字
            std::istringstream lineStream(line);
            lineStream >> pkg.name;
            
            // 2. 检查剩余部分是否有 ver =
            std::string restOfLine;
            std::getline(lineStream, restOfLine);
            
            std::regex verReg(R"(ver\s*=\s*([^s]+))"); // 捕获 ver = 之后直到 sha256 之前的内容
            std::smatch verMatch;
            if (std::regex_search(restOfLine, verMatch, verReg)) {
                pkg.version = verMatch[1].str();
                // 清理尾部空格
                pkg.version.erase(pkg.version.find_last_not_of(" \t\r\n") + 1);
                pkg.hasVersion = true;
            }
            
            std::regex shaReg(R"(sha256\s*=\s*(\S+))");
            std::smatch shaMatch;
            if (std::regex_search(restOfLine, shaMatch, shaReg)) {
                pkg.sha256 = shaMatch[1].str();
                pkg.hasSha256 = true;
            }
            
            if (!pkg.name.empty()) {
                packages.push_back(pkg);
            }
        }
    }

    if (packages.empty()) {
        std::cout << COLOR_YELLOW << "No packages found." << COLOR_RESET << std::endl;
        return true;
    }

    std::cout << COLOR_BOLD << "Found " << packages.size() << " package(s):" << COLOR_RESET << std::endl;
    bool allSuccess = true;
    for (const auto& pkg : packages) {
        std::cout << COLOR_CYAN << "Installing: " << pkg.name;
        if (pkg.hasVersion) std::cout << " @ " << pkg.version;
        std::cout << COLOR_RESET << std::endl;
        
        if (!installModuleWithGit(pkg.name, pkg.version, pkg.sha256)) {
            std::cerr << COLOR_RED << "Failed: " << pkg.name << COLOR_RESET << std::endl;
            allSuccess = false;
        }
    }
    return allSuccess;
}

bool runProject() {
    // 1. Find toml
    std::string tomlFile = "";
    for (const auto& entry : fs::directory_iterator(".")) {
        if (entry.is_regular_file() && entry.path().extension() == ".toml") {
            tomlFile = entry.path().string();
            break;
        }
    }
    if (tomlFile.empty()) {
        std::cerr << COLOR_RED << "Error: No .toml found." << COLOR_RESET << std::endl;
        return false;
    }

    // 2. Parse dependencies (reuse logic from installPackages but simplified for checking)
    std::ifstream file(tomlFile);
    std::string content((std::istreambuf_iterator<char>(file)), std::istreambuf_iterator<char>());
    file.close();

    std::vector<PackageInfo> requiredPackages;
    std::regex reg(R"(need\s*=\s*\{([^}]*)\})");
    std::smatch match;
    if (std::regex_search(content, match, reg)) {
        std::string block = match[1].str();
        std::istringstream iss(block);
        std::string line;
        while (std::getline(iss, line)) {
            if (line.empty() || line.find('#') != std::string::npos) continue;
            if (!line.empty() && line.back() == ',') line.pop_back();
            line.erase(0, line.find_first_not_of(" \t"));
            if (line.empty()) continue;
            
            PackageInfo pkg;
            pkg.hasVersion = false; pkg.hasSha256 = false;
            std::istringstream ls(line);
            ls >> pkg.name;
            std::string rest; std::getline(ls, rest);
            
            std::regex verReg(R"(ver\s*=\s*([^s]+))");
            std::smatch vm;
            if (std::regex_search(rest, vm, verReg)) { pkg.version = vm[1].str(); pkg.hasVersion = true; }
            
            std::regex shaReg(R"(sha256\s*=\s*(\S+))");
            std::smatch sm;
            if (std::regex_search(rest, sm, shaReg)) { pkg.sha256 = sm[1].str(); pkg.hasSha256 = true; }
            
            if (!pkg.name.empty()) requiredPackages.push_back(pkg);
        }
    }

    // 3. Check local mods
    std::vector<std::string> installedMods;
    fs::path modsPath = fs::path(MODS_DIR);
    if (fs::exists(modsPath)) {
        for (const auto& entry : fs::directory_iterator(modsPath)) {
            if (entry.is_regular_file() && entry.path().extension() == ".js") {
                installedMods.push_back(entry.path().filename().string());
            }
        }
    }

    // 4. Identify missing
    std::vector<PackageInfo> missingPackages;
    for (const auto& req : requiredPackages) {
        bool found = false;
        for (const auto& inst : installedMods) {
            std::string lowerReq = req.name;
            std::string lowerInst = inst;
            std::transform(lowerReq.begin(), lowerReq.end(), lowerReq.begin(), ::tolower);
            std::transform(lowerInst.begin(), lowerInst.end(), lowerInst.begin(), ::tolower);
            if (lowerInst.find(lowerReq) != std::string::npos || lowerReq.find(lowerInst) != std::string::npos) {
                found = true;
                break;
            }
        }
        if (!found) missingPackages.push_back(req);
    }

    if (!missingPackages.empty()) {
        std::cout << COLOR_YELLOW << "[Info] Installing missing dependencies..." << COLOR_RESET << std::endl;
        for (const auto& pkg : missingPackages) {
            installModuleWithGit(pkg.name, pkg.version, pkg.sha256);
        }
    } else {
        std::cout << COLOR_GREEN << "[Info] Dependencies satisfied." << COLOR_RESET << std::endl;
    }

    // 5. Run
    std::string compilerPath = "Quernc.exe";
    if (!fs::exists(compilerPath)) {
        std::cerr << COLOR_RED << "Error: Quernc.exe not found." << COLOR_RESET << std::endl;
        return false;
    }
    fs::path mainScript = fs::path("src") / "main.q";
    if (!fs::exists(mainScript)) {
        std::cerr << COLOR_RED << "Error: src/main.q not found." << COLOR_RESET << std::endl;
        return false;
    }

    std::string runCmd = compilerPath + " --Run src\\main.q";
    std::cout << COLOR_BOLD << "\n[Running] " << runCmd << COLOR_RESET << std::endl;
    std::cout << "----------------------------------------" << std::endl;

#ifdef _WIN32
    FILE* pipe = _popen(runCmd.c_str(), "r");
#else
    FILE* pipe = popen(runCmd.c_str(), "r");
#endif
    if (!pipe) return false;

    char buffer[256];
    while (fgets(buffer, sizeof(buffer), pipe) != NULL) {
        std::cout << buffer;
        fflush(stdout);
    }
#ifdef _WIN32
    int status = _pclose(pipe);
#else
    int status = pclose(pipe);
#endif
    std::cout << "----------------------------------------" << std::endl;
    return status == 0;
}
