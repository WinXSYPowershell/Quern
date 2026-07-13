#include <windows.h>
#include <iostream>
#include <fstream>
#include <vector>
#include <string>
#include <filesystem>
#include <sstream>
#include <iomanip>
#include <algorithm>

namespace fs = std::filesystem;

// Helper function to execute command and check result
bool ExecuteCommand(const std::string& cmd) {
    int result = system(cmd.c_str());
    return result == 0;
}

// Check if tools are available
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

// Generate the Stub.c content
std::string GenerateStubCode() {
    // Note: The stub now extracts main.q to the same directory as the interpreter
    std::string code = 
"#include <windows.h>\n"
"#include <stdio.h>\n"
"#include <direct.h>\n"
"#include <string.h>\n"
"#include <stdlib.h>\n"
"\n"
"void GetTempQpsDir(char* buffer, size_t size) {\n"
"    char tempPath[MAX_PATH];\n"
"    GetTempPath(MAX_PATH, tempPath);\n"
"    snprintf(buffer, size, \"%sqps\", tempPath);\n"
"}\n"
"\n"
"BOOL ExtractResourceToFile(HINSTANCE hInstance, WORD resourceId, LPCSTR lpFilename) {\n"
"    HRSRC hRes = FindResource(hInstance, MAKEINTRESOURCE(resourceId), RT_RCDATA);\n"
"    if (!hRes) {\n"
"        fprintf(stderr, \"Error: Resource %d not found.\\n\", resourceId);\n"
"        return FALSE;\n"
"    }\n"
"\n"
"    HGLOBAL hGlob = LoadResource(hInstance, hRes);\n"
"    if (!hGlob) {\n"
"        fprintf(stderr, \"Error: Failed to load resource %d.\\n\", resourceId);\n"
"        return FALSE;\n"
"    }\n"
"\n"
"    LPVOID lpResLock = LockResource(hGlob);\n"
"    DWORD dwSize = SizeofResource(hInstance, hRes);\n"
"    if (!lpResLock || dwSize == 0) {\n"
"        fprintf(stderr, \"Error: Failed to lock resource %d.\\n\", resourceId);\n"
"        return FALSE;\n"
"    }\n"
"\n"
"    HANDLE hFile = CreateFile(lpFilename, GENERIC_WRITE, 0, NULL, CREATE_ALWAYS, FILE_ATTRIBUTE_NORMAL, NULL);\n"
"    if (hFile == INVALID_HANDLE_VALUE) {\n"
"        fprintf(stderr, \"Error: Failed to create file %s. Error: %lu\\n\", lpFilename, GetLastError());\n"
"        return FALSE;\n"
"    }\n"
"\n"
"    DWORD dwWritten = 0;\n"
"    if (!WriteFile(hFile, lpResLock, dwSize, &dwWritten, NULL) || dwWritten != dwSize) {\n"
"        fprintf(stderr, \"Error: Failed to write file %s.\\n\", lpFilename);\n"
"        CloseHandle(hFile);\n"
"        return FALSE;\n"
"    }\n"
"    CloseHandle(hFile);\n"
"\n"
"    return TRUE;\n"
"}\n"
"\n"
"BOOL CALLBACK EnumResNameProc(HMODULE hModule, LPCSTR lpType, LPSTR lpName, LONG_PTR lParam) {\n"
"    if (IS_INTRESOURCE(lpName)) {\n"
"        int id = (int)(intptr_t)lpName;\n"
"        if (id >= 103) {\n"
"            char* cwd = (char*)lParam;\n"
"            char filename[MAX_PATH];\n"
"            snprintf(filename, MAX_PATH, \"%s\\\\mods\\\\mod_%d.js\", cwd, id);\n"
"            \n"
"            char modsDir[MAX_PATH];\n"
"            snprintf(modsDir, MAX_PATH, \"%s\\\\mods\", cwd);\n"
"            _mkdir(modsDir);\n"
"\n"
"            if (!ExtractResourceToFile(hModule, id, filename)) {\n"
"                fprintf(stderr, \"Failed to extract JS resource %d\\n\", id);\n"
"            }\n"
"        }\n"
"    }\n"
"    return TRUE;\n"
"}\n"
"\n"
"int main() {\n"
"    printf(\"Quern Runtime Stub Starting...\\n\");\n"
"\n"
"    char cwd[MAX_PATH];\n"
"    GetTempQpsDir(cwd, MAX_PATH);\n"
"    \n"
"    if (_mkdir(cwd) != 0 && errno != EEXIST) {\n"
"        fprintf(stderr, \"Failed to create temp directory: %s\\n\", cwd);\n"
"        return 1;\n"
"    }\n"
"\n"
"    // 1. Extract QuernInterpreter.zip (ID 102) FIRST to get the interpreter\n"
"    char zipPath[MAX_PATH];\n"
"    snprintf(zipPath, MAX_PATH, \"%s\\\\QuernInterpreter.zip\", cwd);\n"
"    if (!ExtractResourceToFile(GetModuleHandle(NULL), 102, zipPath)) {\n"
"        return 1;\n"
"    }\n"
"\n"
"    // 2. Unzip using PowerShell silently\n"
"    char psCmd[4096];\n"
"    snprintf(psCmd, sizeof(psCmd), \n"
"             \"powershell -NoProfile -NonInteractive -Command \\\"Expand-Archive -Path '%s' -DestinationPath '%s' -Force\\\"\",\n"
"             zipPath, cwd);\n"
"    \n"
"    printf(\"Unzipping interpreter...\\n\");\n"
"    int psResult = system(psCmd);\n"
"    if (psResult != 0) {\n"
"        fprintf(stderr, \"Error: Failed to unzip QuernInterpreter.zip. PowerShell exited with code %d\\n\", psResult);\n"
"        DeleteFile(zipPath);\n"
"        return 1;\n"
"    }\n"
"    \n"
"    if (!DeleteFile(zipPath)) {\n"
"        fprintf(stderr, \"Warning: Could not delete temporary zip file.\\n\");\n"
"    }\n"
"\n"
"    // 3. Extract .q file (ID 101) directly to the Interpreter's directory (cwd)\n"
"    // This ensures main.q is alongside QuernInterpreter.exe\n"
"    char qPath[MAX_PATH];\n"
"    snprintf(qPath, MAX_PATH, \"%s\\\\main.q\", cwd);\n"
"    if (!ExtractResourceToFile(GetModuleHandle(NULL), 101, qPath)) {\n"
"        return 1;\n"
"    }\n"
"\n"
"    // 4. Extract JS files (IDs >= 103) into mods subfolder\n"
"    EnumResourceNames(GetModuleHandle(NULL), RT_RCDATA, EnumResNameProc, (LONG_PTR)cwd);\n"
"\n"
"    // 5. Find and Launch QuernInterpreter.exe with --Run flag\n"
"    char interpPath[MAX_PATH];\n"
"    snprintf(interpPath, MAX_PATH, \"%s\\\\QuernInterpreter.exe\", cwd);\n"
"    \n"
"    if (GetFileAttributes(interpPath) == INVALID_FILE_ATTRIBUTES) {\n"
"         fprintf(stderr, \"Error: QuernInterpreter.exe not found after extraction in %s\\n\", cwd);\n"
"         return 1;\n"
"    }\n"
"\n"
"    printf(\"Launching Interpreter...\\n\");\n"
"    \n"
"    STARTUPINFO si;\n"
"    PROCESS_INFORMATION pi;\n"
"    ZeroMemory(&si, sizeof(si));\n"
"    si.cb = sizeof(si);\n"
"    ZeroMemory(&pi, sizeof(pi));\n"
"\n"
"    // Construct command: QuernInterpreter.exe --Run main.q\n"
"    char cmdLine[MAX_PATH * 3];\n"
"    snprintf(cmdLine, sizeof(cmdLine), \"\\\"%s\\\" --Run \\\"main.q\\\"\", interpPath);\n"
"\n"
"    if (CreateProcess(NULL, cmdLine, NULL, NULL, FALSE, 0, NULL, cwd, &si, &pi)) {\n"
"        WaitForSingleObject(pi.hProcess, INFINITE);\n"
"        CloseHandle(pi.hProcess);\n"
"        CloseHandle(pi.hThread);\n"
"    } else {\n"
"        fprintf(stderr, \"Failed to launch interpreter. Error: %lu\\n\", GetLastError());\n"
"        return 1;\n"
"    }\n"
"\n"
"    return 0;\n"
"}\n";
    return code;
}

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
        
        // Check Zip
        std::string zipSourcePath = "Packger_ZIP\\QuernInterpreter.zip";
        if (!fs::exists(zipSourcePath)) {
            std::cerr << "Error: " << zipSourcePath << " not found." << std::endl;
            return 1;
        }
        
        // Collect JS files
        std::vector<std::string> jsFilePaths;
        std::string modsDir = "Mods";
        if (fs::exists(modsDir) && fs::is_directory(modsDir)) {
            for (const auto& entry : fs::directory_iterator(modsDir)) {
                if (entry.is_regular_file() && entry.path().extension() == ".js") {
                    jsFilePaths.push_back(entry.path().string());
                }
            }
        }
        std::cout << "Found " << jsFilePaths.size() << " JS files in Mods directory." << std::endl;

        // 2. Generate .rc file using PATHS
        std::string rcFilename = "temp_resource.rc";
        std::ofstream rcFile(rcFilename);
        if (!rcFile.is_open()) {
            throw std::runtime_error("Failed to create RC file.");
        }

        rcFile << "#include <windows.h>\n\n";
        
        // ID 101: .q file
        rcFile << "101 RCDATA \"" << qFile << "\"\n\n";

        // ID 102: QuernInterpreter.zip
        rcFile << "102 RCDATA \"" << zipSourcePath << "\"\n\n";

        // ID 103+: JS files
        int currentId = 103;
        for (const auto& jsPath : jsFilePaths) {
            rcFile << currentId << " RCDATA \"" << jsPath << "\"\n\n";
            currentId++;
        }
        
        rcFile.close();
        std::cout << "Generated " << rcFilename << std::endl;

        // 3. Compile .rc to .res
        std::string resFilename = "temp_resource.res";
        std::string rcCmd = "rc /fo \"" + resFilename + "\" \"" + rcFilename + "\"";
        std::cout << "Compiling resources..." << std::endl;
        if (!ExecuteCommand(rcCmd)) {
            std::cerr << "Error: Failed to compile resources." << std::endl;
            return 1;
        }

        // 4. Generate Stub.c
        std::string stubFilename = "Stub.c";
        std::ofstream stubFile(stubFilename);
        if (!stubFile.is_open()) {
            throw std::runtime_error("Failed to create Stub.c");
        }
        stubFile << GenerateStubCode();
        stubFile.close();
        std::cout << "Generated " << stubFilename << std::endl;

        // 5. Determine Output Filename
        fs::path qPathObj(qFile);
        std::string outputExe = qPathObj.stem().string() + ".exe";
        
        // 6. Compile Stub.c with .res to final executable
        std::string finalClCmd = "cl /MT \"" + stubFilename + "\" \"" + resFilename + "\" /Fe\"" + outputExe + "\"";
        
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
