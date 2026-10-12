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