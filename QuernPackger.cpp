#include <windows.h>
#include <iostream>
#include <fstream>
#include <vector>
#include <string>
#include <filesystem>
#include <sstream>
#include <iomanip>
#include <algorithm>
#include <regex>
#include <set>
#include <map>
#include <functional> // Required for std::function

namespace fs = std::filesystem;

// ==========================================
// Global Configuration & Data Structures
// ==========================================

struct ResourceEntry {
    int id;
    std::string sourcePath; // Path on disk during compilation
    std::string targetPath; // Relative path inside the app during runtime
};

std::vector<ResourceEntry> g_Resources;
std::set<std::string> g_ProcessedFiles; // To avoid circular dependencies and duplicates

// ==========================================
// Helper Functions
// ==========================================

bool ExecuteCommand(const std::string& cmd) {
    int result = system(cmd.c_str());
    return result == 0;
}

bool CheckEnvironment() {
    std::cout << "Checking environment..." << std::endl;
    
    if (system("where cl >nul 2>&1") != 0) {
        std::cerr << "Error: cl.exe not found. Please configure Visual Studio build environment." << std::endl;
        return false;
    }
    
    if (system("where rc >nul 2>&1") != 0) {
        std::cerr << "Error: rc.exe not found. Please configure Windows SDK environment." << std::endl;
        return false;
    }
    
    if (system("where powershell >nul 2>&1") != 0) {
        std::cerr << "Error: PowerShell not found." << std::endl;
        return false;
    }

    std::cout << "Environment check passed." << std::endl;
    return true;
}

// ==========================================
// Stub Code Generation
// ==========================================

void GenerateStubAndHeader(const std::vector<ResourceEntry>& resources) {
    // 1. Generate ResourceMap.h
    std::string headerFilename = "ResourceMap.h";
    std::ofstream headerFile(headerFilename);
    if (!headerFile.is_open()) throw std::runtime_error("Failed to create ResourceMap.h");

    headerFile << "#pragma once\n";
    headerFile << "#include <windows.h>\n\n";
    headerFile << "typedef struct {\n";
    headerFile << "    UINT id;\n";
    headerFile << "    LPCSTR path;\n";
    headerFile << "} ResourceMapEntry;\n\n";
    
    headerFile << "static const ResourceMapEntry ResourceMap[] = {\n";
    for (const auto& res : resources) {
        // Escape backslashes for C string
        std::string escapedPath = res.targetPath;
        size_t pos = 0;
        while ((pos = escapedPath.find('\\', pos)) != std::string::npos) {
            escapedPath.replace(pos, 1, "\\\\");
            pos += 2;
        }
        headerFile << "    { " << res.id << ", \"" << escapedPath << "\" },\n";
    }
    headerFile << "    { 0, NULL } // End marker\n";
    headerFile << "};\n\n";
    headerFile << "#define RESOURCE_MAP_SIZE " << resources.size() << "\n";
    headerFile.close();

    // 2. Generate Stub.c
    std::string stubFilename = "Stub.c";
    std::ofstream stubFile(stubFilename);
    if (!stubFile.is_open()) throw std::runtime_error("Failed to create Stub.c");

    // Using raw string literal R"(...)" is safest for embedding C code in C++
    const char* stubCode = R"(
#include <windows.h>
#include <stdio.h>
#include <direct.h>
#include <string.h>
#include <stdlib.h>
#include "ResourceMap.h"

void GetTempQpsDir(char* buffer, size_t size) {
    char tempPath[MAX_PATH];
    GetTempPath(MAX_PATH, tempPath);
    snprintf(buffer, size, "%sqps", tempPath);
}

BOOL EnsureDirectoryExists(const char* path) {
    char tmp[MAX_PATH];
    snprintf(tmp, MAX_PATH, "%s", path);
    
    // Replace forward slashes with backslashes
    for (char* p = tmp; *p; p++) {
        if (*p == '/') *p = '\\';
    }

    // Simple recursive mkdir implementation
    for (char* p = tmp + 1; *p; p++) {
        if (*p == '\\') {
            *p = '\0';
            _mkdir(tmp);
            *p = '\\';
        }
    }
    _mkdir(tmp);
    return TRUE;
}

BOOL ExtractResourceToFile(HINSTANCE hInstance, WORD resourceId, LPCSTR lpFilename) {
    HRSRC hRes = FindResource(hInstance, MAKEINTRESOURCE(resourceId), RT_RCDATA);
    if (!hRes) {
        fprintf(stderr, "Error: Resource %d not found.\n", resourceId);
        return FALSE;
    }

    HGLOBAL hGlob = LoadResource(hInstance, hRes);
    if (!hGlob) {
        fprintf(stderr, "Error: Failed to load resource %d.\n", resourceId);
        return FALSE;
    }

    LPVOID lpResLock = LockResource(hGlob);
    DWORD dwSize = SizeofResource(hInstance, hRes);
    if (!lpResLock || dwSize == 0) {
        fprintf(stderr, "Error: Failed to lock resource %d.\n", resourceId);
        return FALSE;
    }

    // Ensure directory exists
    char dirPath[MAX_PATH];
    strncpy(dirPath, lpFilename, MAX_PATH - 1);
    dirPath[MAX_PATH - 1] = '\0';
    
    char* lastSlash = strrchr(dirPath, '\\');
    if (!lastSlash) lastSlash = strrchr(dirPath, '/');
    
    if (lastSlash) {
        *lastSlash = '\0';
        EnsureDirectoryExists(dirPath);
    }

    HANDLE hFile = CreateFile(lpFilename, GENERIC_WRITE, 0, NULL, CREATE_ALWAYS, FILE_ATTRIBUTE_NORMAL, NULL);
    if (hFile == INVALID_HANDLE_VALUE) {
        fprintf(stderr, "Error: Failed to create file %s. Error: %lu\n", lpFilename, GetLastError());
        return FALSE;
    }

    DWORD dwWritten = 0;
    if (!WriteFile(hFile, lpResLock, dwSize, &dwWritten, NULL) || dwWritten != dwSize) {
        fprintf(stderr, "Error: Failed to write file %s.\n", lpFilename);
        CloseHandle(hFile);
        return FALSE;
    }
    CloseHandle(hFile);

    return TRUE;
}

int main() {
    printf("Quern Runtime Stub Starting...\n");

    char cwd[MAX_PATH];
    GetTempQpsDir(cwd, MAX_PATH);
    
    if (_mkdir(cwd) != 0 && errno != EEXIST) {
        fprintf(stderr, "Failed to create temp directory: %s\n", cwd);
        return 1;
    }

    // 1. Extract QuernInterpreter.zip (ID 102) FIRST
    char zipPath[MAX_PATH];
    snprintf(zipPath, MAX_PATH, "%s\\QuernInterpreter.zip", cwd);
    if (!ExtractResourceToFile(GetModuleHandle(NULL), 102, zipPath)) {
        return 1;
    }

    // 2. Unzip using PowerShell silently
    char psCmd[4096];
    snprintf(psCmd, sizeof(psCmd), 
             "powershell -NoProfile -NonInteractive -Command \"Expand-Archive -Path '%s' -DestinationPath '%s' -Force\"",
             zipPath, cwd);
    
    printf("Unzipping interpreter...\n");
    int psResult = system(psCmd);
    if (psResult != 0) {
        fprintf(stderr, "Error: Failed to unzip QuernInterpreter.zip. PowerShell exited with code %d\n", psResult);
        DeleteFile(zipPath);
        return 1;
    }
    
    if (!DeleteFile(zipPath)) {
        fprintf(stderr, "Warning: Could not delete temporary zip file.\n");
    }

    // 3. Extract all other resources based on ResourceMap
    for (int i = 0; i < RESOURCE_MAP_SIZE; i++) {
        UINT id = ResourceMap[i].id;
        LPCSTR relPath = ResourceMap[i].path;
        
        // Skip ID 102 as it's already handled
        if (id == 102) continue;

        char fullPath[MAX_PATH];
        snprintf(fullPath, MAX_PATH, "%s\\%s", cwd, relPath);
        
        if (!ExtractResourceToFile(GetModuleHandle(NULL), id, fullPath)) {
            fprintf(stderr, "Failed to extract resource %d (%s)\n", id, relPath);
        }
    }

    // 4. Find and Launch QuernInterpreter.exe
    char interpPath[MAX_PATH];
    snprintf(interpPath, MAX_PATH, "%s\\QuernInterpreter.exe", cwd);
    
    if (GetFileAttributes(interpPath) == INVALID_FILE_ATTRIBUTES) {
         fprintf(stderr, "Error: QuernInterpreter.exe not found after extraction in %s\n", cwd);
         return 1;
    }

    printf("Launching Interpreter...\n");
    
    STARTUPINFO si;
    PROCESS_INFORMATION pi;
    ZeroMemory(&si, sizeof(si));
    si.cb = sizeof(si);
    ZeroMemory(&pi, sizeof(pi));

    // Construct command: QuernInterpreter.exe --Run main.q
    char cmdLine[MAX_PATH * 3];
    snprintf(cmdLine, sizeof(cmdLine), "\"%s\" --Run \"main.q\"", interpPath);

    if (CreateProcess(NULL, cmdLine, NULL, NULL, FALSE, 0, NULL, cwd, &si, &pi)) {
        WaitForSingleObject(pi.hProcess, INFINITE);
        CloseHandle(pi.hProcess);
        CloseHandle(pi.hThread);
    } else {
        fprintf(stderr, "Failed to launch interpreter. Error: %lu\n", GetLastError());
        return 1;
    }

    return 0;
}
)";

    stubFile << stubCode;
    stubFile.close();
}

// ==========================================
// Main Logic
// ==========================================

int main(int argc, char* argv[]) {
    if (argc < 3) {
        std::cout << "Usage: QuernPackger.exe --Build <file.q>" << std::endl;
        return 1;
    }

    std::string action = argv[1];
    std::string qFile = argv[2];

    if (action != "--Build") {
        std::cout << "Unknown action: " << action << std::endl;
        std::cout << "Usage: QuernPackger.exe --Build <file.q>" << std::endl;
        return 1;
    }

    if (!fs::exists(qFile)) {
        std::cerr << "Error: File '" << qFile << "' not found." << std::endl;
        return 1;
    }

    if (!CheckEnvironment()) {
        return 1;
    }

    std::cout << "Starting packing process for: " << qFile << std::endl;

    try {
        // 1. Prepare Paths
        std::string zipSourcePath = "Packger_ZIP\\QuernInterpreter.zip";
        if (!fs::exists(zipSourcePath)) {
            std::cerr << "Error: " << zipSourcePath << " not found." << std::endl;
            return 1;
        }

        // 2. Analyze Dependencies
        std::string baseDir = fs::current_path().string();
        std::string mainQAbs = fs::absolute(qFile).string();
        
        std::cout << "Analyzing dependencies..." << std::endl;
        
        g_ProcessedFiles.clear();
        std::vector<std::pair<std::string, std::string>> allDeps; // <AbsPath, RelPath>
        
        // Recursive lambda to collect deps
        std::function<void(const std::string&, const std::string&)> collectDeps;
        collectDeps = [&](const std::string& filePath, const std::string& base) {
            std::string absPath = fs::absolute(filePath).string();
            
            if (g_ProcessedFiles.count(absPath)) return;
            if (!fs::exists(absPath)) {
                std::cerr << "Warning: Dependency not found: " << absPath << std::endl;
                return;
            }
            
            g_ProcessedFiles.insert(absPath);
            
            // Calculate Target Path
            std::string targetPath;
            fs::path p(absPath);
            std::string ext = p.extension().string();
            
            if (ext == ".js") {
                // For JS files: Force them into "mods/" folder with their original filename
                // Example: I:\Project\Mods\bstring.js -> mods/bstring.js
                targetPath = "mods/" + p.filename().string();
            } else {
                // For .q files: Keep relative structure from base directory
                // Example: I:\Project\libs\utils.q -> libs/utils.q
                targetPath = fs::relative(absPath, base).generic_string();
            }

            // Add to list
            allDeps.push_back({absPath, targetPath});
            
            // Only parse .q files for further dependencies
            if (ext != ".q") {
                return;
            }
            
            // Read and parse
            std::ifstream ifs(absPath);
            if(!ifs) {
                std::cerr << "Error: Could not open " << absPath << std::endl;
                return;
            }
            std::string content((std::istreambuf_iterator<char>(ifs)), std::istreambuf_iterator<char>());
            ifs.close();
            
            // Regex to match Import "..." or Include "..."
            std::regex depRegex(R"((?:Import|Include)\s+["']([^"']+)["'])");
            std::sregex_iterator iter(content.begin(), content.end(), depRegex);
            std::sregex_iterator end;
            
            while(iter != end) {
                std::string ref = (*iter)[1].str();
                ++iter;
                
                fs::path currentDir = fs::path(absPath).parent_path();
                fs::path resolved = fs::weakly_canonical(currentDir / ref);
                
                if(fs::exists(resolved)) {
                    collectDeps(resolved.string(), base);
                } else {
                    std::cerr << "Warning: Referenced file not found: " << resolved.string() << std::endl;
                }
            }
        };
        
        collectDeps(mainQAbs, baseDir);
        
        // 3. Build Resource List
        g_Resources.clear();
        
        // ID 101: Main .q file (always named main.q in runtime)
        g_Resources.push_back({101, mainQAbs, "main.q"});
        
        // ID 102: Interpreter Zip
        g_Resources.push_back({102, zipSourcePath, "QuernInterpreter.zip"});
        
        // IDs 103+: Dependencies
        int currentId = 103;
        
        for (auto& dep : allDeps) {
            if (dep.first == mainQAbs) continue; // Skip main, already added as 101
            
            g_Resources.push_back({currentId++, dep.first, dep.second});
        }

        std::cout << "Found " << (g_Resources.size() - 2) << " dependencies." << std::endl;
        for(const auto& r : g_Resources) {
            if(r.id > 102) {
                std::cout << "  ID " << r.id << ": " << r.sourcePath << " -> " << r.targetPath << std::endl;
            }
        }

        // 4. Generate .rc file
        std::string rcFilename = "temp_resource.rc";
        std::ofstream rcFile(rcFilename);
        if (!rcFile.is_open()) throw std::runtime_error("Failed to create RC file.");

        rcFile << "#include <windows.h>\n\n";
        
        for (const auto& res : g_Resources) {
            // Escape quotes in path if any
            std::string path = res.sourcePath;
            rcFile << res.id << " RCDATA \"" << path << "\"\n";
        }
        
        rcFile.close();
        std::cout << "Generated " << rcFilename << std::endl;

        // 5. Compile .rc to .res
        std::string resFilename = "temp_resource.res";
        std::string rcCmd = "rc /fo \"" + resFilename + "\" \"" + rcFilename + "\"";
        std::cout << "Compiling resources..." << std::endl;
        if (!ExecuteCommand(rcCmd)) {
            std::cerr << "Error: Failed to compile resources." << std::endl;
            return 1;
        }

        // 6. Generate Stub.c and ResourceMap.h
        std::cout << "Generating Stub code..." << std::endl;
        GenerateStubAndHeader(g_Resources);

        // 7. Determine Output Filename
        fs::path qPathObj(qFile);
        std::string outputExe = qPathObj.stem().string() + ".exe";
        
        // 8. Compile Stub.c with .res to final executable
        std::string finalClCmd = "cl /MT Stub.c \"" + resFilename + "\" /Fe\"" + outputExe + "\"";
        
        std::cout << "Compiling final executable: " << outputExe << "..." << std::endl;
        if (!ExecuteCommand(finalClCmd)) {
            std::cerr << "Error: Failed to compile final executable." << std::endl;
            return 1;
        }

        std::cout << "----------------------------------------" << std::endl;
        std::cout << "Packing Successful!" << std::endl;
        std::cout << "Output: " << outputExe << std::endl;
        std::cout << "----------------------------------------" << std::endl;

    } catch (const std::exception& e) {
        std::cerr << "Exception: " << e.what() << std::endl;
        return 1;
    }

    return 0;
}
