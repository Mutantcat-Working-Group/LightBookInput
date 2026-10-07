//! 真 Rust Server 与 Fcitx 默认面板的协议/中文输入闭环，避免 mock 隐藏版本漂移。
#include "lightbookinput.h"
#include "support/context.h"
#include <fcitx/inputcontextmanager.h>
#include <fcitx/inputpanel.h>
#include <cassert>
#include <chrono>
#include <filesystem>
#include <thread>
#include <sys/wait.h>
#include <signal.h>
#include <unistd.h>
int main(int argc, char **argv) {
    assert(argc == 3);
    char directory[] = "/tmp/lightbookinput-real-XXXXXX";
    assert(mkdtemp(directory));
    std::string socketPath = std::string(directory) + "/server.sock";
    std::string dictionary = std::string(argv[2]) + "/assets/sample/dict.tsv";
    setenv("LIGHTBOOKINPUT_SOCKET", socketPath.c_str(), 1);
    auto sampleDir = std::filesystem::path(directory) / "assets/sample";
    std::filesystem::create_directories(sampleDir);
    std::filesystem::copy_file(std::filesystem::path(argv[2]) / "assets/sample/english.tsv", sampleDir / "english.tsv");
    setenv("LIGHTBOOKINPUT_RESOURCES", directory, 1);
    setenv("LIGHTBOOKINPUT_DICT", dictionary.c_str(), 1);
    setenv("XDG_CONFIG_HOME", directory, 1);
    setenv("XDG_DATA_HOME", directory, 1);
    setenv("XDG_STATE_HOME", directory, 1);
    unsetenv("DBUS_SESSION_BUS_ADDRESS");
    auto server = fork();
    assert(server >= 0);
    if (server == 0) { execl(argv[1], argv[1], nullptr); _exit(127); }
    for (int i = 0; i < 500 && !std::filesystem::exists(socketPath); ++i) std::this_thread::sleep_for(std::chrono::milliseconds(10));
    assert(std::filesystem::exists(socketPath));
    {
        char program[] = "lightbookinput-test"; char disable[] = "--disable=all";
        char *arguments[] = {program, disable, nullptr};
        fcitx::Instance instance(2, arguments);
        instance.initialize();
        fcitx::LightBookInputEngine engine(&instance.addonManager());
        Context context(instance.inputContextManager());
        context.focusIn(); // 全空 capability 应正常输入。
        for (char c : std::string("nihao")) assert(engine.process(&context, fcitx::Key(static_cast<fcitx::KeySym>(c))));
        auto list = context.inputPanel().candidateList();
        assert(list && list->size() > 0 && list->candidate(0).text().toString() == "你好");
        list->candidate(0).select(&context);
        assert(context.committed == "你好");
        list->candidate(0).select(&context);
        assert(context.committed == "你好");
        // 同一上下文失焦 / 回焦保留 Server 的英文模式；框架 reset 只清输入。
        engine.process(&context, fcitx::Key(FcitxKey_Shift_L));
        assert(engine.process(&context, fcitx::Key(FcitxKey_Shift_L), true));
        context.focusOut();
        fcitx::InputMethodEntry entry("lightbookinput", "lightbookinput", "zh_CN", "lightbookinput");
        fcitx::FocusOutEvent out(&context);
        engine.deactivate(entry, out);
        context.focusIn();
        // 英文模式缺省直插：字母直接上屏，不出候选、不留拼音行。
        assert(engine.process(&context, fcitx::Key(FcitxKey_h)));
        assert(context.committed == "你好h");
        assert(context.inputPanel().empty());
        fcitx::InputContextEvent reset(&context, fcitx::EventType::InputContextReset);
        engine.reset(entry, reset);
        assert(context.inputPanel().empty());
        context.setCapabilityFlags(fcitx::CapabilityFlag::Password);
        assert(!engine.process(&context, fcitx::Key(FcitxKey_n)));
        assert(context.inputPanel().empty());
    }
    kill(server, SIGTERM);
    int status = 0;
    waitpid(server, &status, 0);
    assert(WIFEXITED(status) && WEXITSTATUS(status) == 0);
    std::filesystem::remove_all(directory);
}
