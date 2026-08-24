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

//   SHA256 实现 (轻量级单文件)  
// 基于 public domain implementation
class SHA256 {
protected:
    typedef unsigned char uint8;
    typedef unsigned int uint32;
    typedef unsigned long long uint64;

    const static uint32 sha256_k[];
    static const unsigned int SHA224_256_BLOCK_SIZE = (512 / 8);
public:
    void init();
    void update(const unsigned char *message, unsigned int len);
    void final(unsigned char *digest);
    static const unsigned int DIGEST_SIZE = (256 / 8);

protected:
    void transform(const unsigned char *message, unsigned int block_nb);
    unsigned int m_tot_len;
    unsigned int m_len;
    unsigned char m_block[2 * SHA224_256_BLOCK_SIZE];
    uint32 m_h[8];
};

#define SHA2_SHFR(x, n)    (x >> n)
#define SHA2_ROTR(x, n)   ((x >> n) | (x << ((sizeof(x) << 3) - n)))
#define SHA2_ROTL(x, n)   ((x << n) | (x >> ((sizeof(x) << 3) - n)))
#define SHA2_CH(x, y, z)  ((x & y) ^ (~x & z))
#define SHA2_MAJ(x, y, z) ((x & y) ^ (x & z) ^ (y & z))
#define SHA256_F1(x) (SHA2_ROTR(x,  2) ^ SHA2_ROTR(x, 13) ^ SHA2_ROTR(x, 22))
#define SHA256_F2(x) (SHA2_ROTR(x,  6) ^ SHA2_ROTR(x, 11) ^ SHA2_ROTR(x, 25))
#define SHA256_F3(x) (SHA2_ROTR(x,  7) ^ SHA2_ROTR(x, 18) ^ SHA2_ROTR(x, 31))
#define SHA256_F4(x) (SHA2_ROTR(x, 17) ^ SHA2_ROTR(x, 19) ^ SHA2_ROTR(x, 24))
#define SHA2_UNPACK32(x, str) { \
    *((str) + 3) = (uint8) ((x)      ); \
    *((str) + 2) = (uint8) ((x) >>  8); \
    *((str) + 1) = (uint8) ((x) >> 16); \
    *((str) + 0) = (uint8) ((x) >> 24); }
#define SHA2_PACK32(str, x) { \
    *(x) =   ((uint32) *((str) + 3)      ) \
           | ((uint32) *((str) + 2) <<  8) \
           | ((uint32) *((str) + 1) << 16) \
           | ((uint32) *((str) + 0) << 24); }

const unsigned int SHA256::sha256_k[64] = {
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2
};

void SHA256::init() {
    m_h[0] = 0x6a09e667; m_h[1] = 0xbb67ae85; m_h[2] = 0x3c6ef372; m_h[3] = 0xa54ff53a;
    m_h[4] = 0x510e527f; m_h[5] = 0x9b05688c; m_h[6] = 0x1f83d9ab; m_h[7] = 0x5be0cd19;
    m_len = 0;
    m_tot_len = 0;
}

void SHA256::update(const unsigned char *message, unsigned int len) {
    unsigned int block_nb;
    unsigned int new_len, rem_len, tmp_len;
    const unsigned char *shifted_message;
    tmp_len = SHA224_256_BLOCK_SIZE - m_len;
    rem_len = len < tmp_len ? len : tmp_len;
    memcpy(&m_block[m_len], message, rem_len);
    if (m_len + len < SHA224_256_BLOCK_SIZE) {
        m_len += len;
        return;
    }
    new_len = len - rem_len;
    block_nb = new_len / SHA224_256_BLOCK_SIZE;
    shifted_message = message + rem_len;
    transform(m_block, 1);
    transform(shifted_message, block_nb);
    rem_len = new_len % SHA224_256_BLOCK_SIZE;
    memcpy(m_block, &shifted_message[block_nb * SHA224_256_BLOCK_SIZE], rem_len);
    m_len = rem_len;
    m_tot_len += (block_nb + 1) * SHA224_256_BLOCK_SIZE;
}

void SHA256::final(unsigned char *digest) {
    unsigned int block_nb;
    unsigned int pm_len;
    unsigned int len_b;
    block_nb = (1 + ((SHA224_256_BLOCK_SIZE - 9) < (m_len % SHA224_256_BLOCK_SIZE)));
    m_len = 0;
    m_tot_len += block_nb * SHA224_256_BLOCK_SIZE;
    memset(&m_block[m_len], 0, block_nb * SHA224_256_BLOCK_SIZE);
    m_block[m_len] = 0x80;
    pm_len = (block_nb * SHA224_256_BLOCK_SIZE) - 8;
    SHA2_UNPACK32(m_tot_len << 3, &m_block[pm_len]);
    transform(m_block, block_nb);
    SHA2_UNPACK32(m_h[0], digest);
    SHA2_UNPACK32(m_h[1], digest + 4);
    SHA2_UNPACK32(m_h[2], digest + 8);
    SHA2_UNPACK32(m_h[3], digest + 12);
    SHA2_UNPACK32(m_h[4], digest + 16);
    SHA2_UNPACK32(m_h[5], digest + 20);
    SHA2_UNPACK32(m_h[6], digest + 24);
    SHA2_UNPACK32(m_h[7], digest + 28);
}

void SHA256::transform(const unsigned char *message, unsigned int block_nb) {
    uint32 w[64];
    uint32 wv[8];
    uint32 t1, t2;
    const unsigned char *sub_block;
    unsigned int i;
    for (i = 0; i < block_nb; i++) {
        sub_block = message + (i << 6);
        for (int j = 0; j < 16; j++) {
            SHA2_PACK32(&sub_block[j << 2], &w[j]);
        }
        for (int j = 16; j < 64; j++) {
            w[j] = SHA256_F4(w[j - 2]) + w[j - 7] + SHA256_F3(w[j - 15]) + w[j - 16];
        }
        for (int j = 0; j < 8; j++) {
            wv[j] = m_h[j];
        }
        for (int j = 0; j < 64; j++) {
            t1 = wv[7] + SHA256_F2(wv[4]) + SHA2_CH(wv[4], wv[5], wv[6]) + sha256_k[j] + w[j];
            t2 = SHA256_F1(wv[0]) + SHA2_MAJ(wv[0], wv[1], wv[2]);
            wv[7] = wv[6];
            wv[6] = wv[5];
            wv[5] = wv[4];
            wv[4] = wv[3] + t1;
            wv[3] = wv[2];
            wv[2] = wv[1];
            wv[1] = wv[0];
            wv[0] = t1 + t2;
        }
        for (int j = 0; j < 8; j++) {
            m_h[j] += wv[j];
        }
    }
}

std::string computeSHA256(const std::string& filename) {
    std::ifstream file(filename, std::ios::binary);
    if (!file.is_open()) return "";

    SHA256 ctx;
    ctx.init();
    
    char buffer[8192];
    while (file.read(buffer, sizeof(buffer))) {
        ctx.update(reinterpret_cast<unsigned char*>(buffer), file.gcount());
    }
    // Process remaining bytes
    if (file.gcount() > 0) {
        ctx.update(reinterpret_cast<unsigned char*>(buffer), file.gcount());
    }

    unsigned char digest[SHA256::DIGEST_SIZE];
    ctx.final(digest);

    std::stringstream ss;
    for (int i = 0; i < SHA256::DIGEST_SIZE; i++) {
        ss << std::hex << std::setw(2) << std::setfill('0') << (int)digest[i];
    }
    return ss.str();
}

//   辅助函数  

void printHelp() {
    std::cout << COLOR_BOLD << "Quern Libraries Manager (QLM) v2.0" << COLOR_RESET << std::endl;
    std::cout << "Usage: qlm [options]" << std::endl;
    std::cout << std::endl;
    std::cout << "Options:" << std::endl;
    std::cout << "  --Install <module_name.js>     Download and install a module (latest version)." << std::endl;
    std::cout << "  --Delete <module_name.js>      Delete a module from the mods folder." << std::endl;
    std::cout << "  --Disable <module_name.js>     Move a module to the 'disabled' folder." << std::endl;
    std::cout << "  --Enable <module_name.js>      Move a module from 'disabled' back to 'mods'." << std::endl;
    std::cout << "  --WebList                      List all available .js files in the remote repository." << std::endl;
    std::cout << "  --ModsList                     List all installed .js files in the local mods folder." << std::endl;
    std::cout << "  --WebSearch <keyword>          Search for modules in the remote repository by filename." << std::endl;
    std::cout << "  --ModsSearch <keyword>         Search for modules in the local mods folder by filename." << std::endl;
    std::cout << "  --NewProject <name>,<ver>      Create a new project with src/main.q and config.toml." << std::endl;
    std::cout << "  --InstallPackage               Read local .toml and install required packages (supports ver/sha256)." << std::endl;
    std::cout << "  --ProjectRun                   Check dependencies, install missing ones, and run the project." << std::endl;
    std::cout << "  --Help                         Show this help message." << std::endl;
}

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
    // 注意：这里假设 versionDate 格式为 YYYY/MM/DD,HH/MM/SS 或类似 git 能识别的日期
    if (!versionDate.empty()) {
        // 将用户输入的逗号分隔日期转换为 git 可能接受的格式，或者尝试直接使用
        // Git 通常接受 "YYYY-MM-DD HH:MM:SS" 或相对时间
        // 为了简化，我们尝试直接传入，如果失败则回退到最新
        std::string formattedDate = versionDate;
        std::replace(formattedDate.begin(), formattedDate.end(), ',', ' '); // 替换逗号为空格
        
        std::string checkoutCmd = "cd \"" + tempDirPath.string() + "\" && git checkout $(git rev-list -n 1 --before=\"" + formattedDate + "\" HEAD)";
        
        // Windows 下 git rev-list 可能需要不同的处理，这里使用更通用的 approach:
        // 先 fetch 所有历史（因为 depth 1 拿不到历史），或者如果用户指定了版本，我们不能用 depth 1
        // 修正：如果指定了版本，必须完整 clone 或者至少 fetch 足够多的历史
        // 为了性能，我们先尝试 depth 1，如果指定了版本，则重新 fetch
        
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
