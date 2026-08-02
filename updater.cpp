#include <iostream>
#include <string>
#include <vector>
#include <cstdlib>
#include <sstream>

#ifdef _WIN32
#include <windows.h>
#include <wincrypt.h>
#pragma comment(lib, "crypt32.lib")

std::string Base64Encode(const std::string& input) {
    DWORD cbEncoded = 0;
    if (!CryptBinaryToStringA((const BYTE*)input.c_str(), (DWORD)input.size(), CRYPT_STRING_BASE64 | CRYPT_STRING_NOCRLF, NULL, &cbEncoded)) {
        return "";
    }
    std::string output(cbEncoded, 0);
    if (!CryptBinaryToStringA((const BYTE*)input.c_str(), (DWORD)input.size(), CRYPT_STRING_BASE64 | CRYPT_STRING_NOCRLF, &output[0], &cbEncoded)) {
        return "";
    }
    return output;
}
#endif

#ifdef __linux__
#include <unistd.h>
#include <sys/wait.h>

std::string execCapture(const char* cmd) {
    std::string result;
    FILE* pipe = popen(cmd, "r");
    if (!pipe) return "";
    char buffer[256];
    while (fgets(buffer, sizeof(buffer), pipe) != nullptr) {
        result += buffer;
    }
    pclose(pipe);
    return result;
}

std::string getLatestTag()
{
    std::string raw = execCapture("git ls-remote --tags https://gitcode.com/2401_86702825/Quern");
    std::istringstream iss(raw);
    std::string line;
    std::string latest;
    while (std::getline(iss, line))
    {
        size_t pos = line.find("refs/tags/");
        if(pos == std::string::npos) continue;
        std::string tag = line.substr(pos + 10);
        if(tag.size() >=2 && tag.substr(tag.size()-2) == "^{}") continue;
        if(latest.empty() || tag > latest)
            latest = tag;
    }
    return latest;
}

int runWget(const std::string& url)
{
    pid_t pid = fork();
    if(pid == 0)
    {
        char* argv[] = {
            (char*)"wget",
            (char*)url.c_str(),
            nullptr
        };
        execvp("wget", argv);
        _exit(127);
    }
    int status;
    waitpid(pid, &status, 0);
    return WEXITSTATUS(status);
}

int runShellCmd(const std::string& cmd)
{
    pid_t pid = fork();
    if(pid == 0)
    {
        char* argv[] = {
            (char*)"/bin/sh",
            (char*)"-c",
            (char*)cmd.c_str(),
            nullptr
        };
        execvp("/bin/sh", argv);
        _exit(127);
    }
    int status;
    waitpid(pid, &status, 0);
    return WEXITSTATUS(status);
}
#endif

void runUpdater() {
    std::cout << "========================================\n";
    std::cout << "       Quern Auto-Updater Started\n";
    std::cout << "========================================\n";

#ifdef _WIN32
    std::cout << "[Updater] Detected Windows OS.\n";
    std::cout << "[Updater] Executing update command...\n";

    std::string psScript =
        "git ls-remote --tags https://gitcode.com/2401_86702825/Quern | "
        "ForEach-Object { $_ -split '\\s+' } | "
        "Where-Object { $_ -match 'refs/tags/' -and $_ -notmatch '\\^{}' } | "
        "ForEach-Object { $_ -replace 'refs/tags/', '' } | "
        "Sort-Object -Descending | Select-Object -First 1 | "
        "ForEach-Object { "
            "$version = $_; "
            "$url = \"https://gitcode.com/2401_86702825/Quern/releases/download/$version/Quern-Linux%26Windows-amd64-LSDK.zip\"; "
            "Write-Host \"New Version: $version\"; "
            "Write-Host \"Download url: $url\"; "
            "Invoke-WebRequest -Uri $url -OutFile \"Quern-Linux&Windows-amd64-LSDK.zip\"; "
            "Expand-Archive -Path \"Quern-Linux&Windows-amd64-LSDK.zip\" -DestinationPath \".\" -Force; "
            "Remove-Item \"Quern-Linux&Windows-amd64-LSDK.zip\"; "
            "Set-Location \"Quern\"; "
            "Write-Host \"[Updater] Update Done.\" "
        "}";

    int wlen = MultiByteToWideChar(CP_UTF8, 0, psScript.c_str(), -1, NULL, 0);
    std::vector<wchar_t> wstr(wlen);
    MultiByteToWideChar(CP_UTF8, 0, psScript.c_str(), -1, wstr.data(), wlen);
    std::string utf16Script((const char*)wstr.data(), (wlen - 1) * sizeof(wchar_t));
    std::string encoded = Base64Encode(utf16Script);
    std::string cmd = "powershell.exe -ExecutionPolicy Bypass -NoProfile -EncodedCommand " + encoded;
    int result = system(cmd.c_str());
    if (result != 0) {
        std::cerr << "[Updater] Error: Command failed with exit code " << result << "\n";
    }

#elif __linux__
    std::cout << "[Updater] Detected Linux OS.\n";
    std::cout << "[Updater] Executing update command...\n";

    std::string ver = getLatestTag();
    if(ver.empty())
    {
        std::cerr << "[Updater] Failed fetch latest tag!\n";
        return;
    }
    std::string url = "https://gitcode.com/2401_86702825/Quern/releases/download/"
                      + ver
                      + "/Quern-Linux%26Windows-amd64-LSDK.zip";

    std::cout << "[DEBUG] Final Download URL: " << url << std::endl;

    int ret = runWget(url);
    if(ret != 0)
    {
        std::cerr << "[Updater] wget failed!\n";
        return;
    }
    std::string postCmd = "unzip -o 'Quern-Linux&Windows-amd64-LSDK.zip' -d . && rm 'Quern-Linux&Windows-amd64-LSDK.zip' && cd Quern";
    runShellCmd(postCmd);
#endif

    std::cout << "========================================\n";
    std::cout << "       Update Process Finished\n";
    std::cout << "========================================\n";
}

int main(int argc, char* argv[]) {
    for (int i = 1; i < argc; ++i) {
        if (std::string(argv[i]) == "--update") {
            runUpdater();
            return 0;
        }
    }
    std::cout << "Usage: updater --update\n";
    return 0;
}