//! 轻书 addon 工厂。
#include "lightbookinput.h"
namespace fcitx {
class LightBookInputFactory final : public AddonFactory {
public:
    AddonInstance *create(AddonManager *manager) override { return new LightBookInputEngine(manager); }
};
}
FCITX_ADDON_FACTORY_V2(lightbookinput, fcitx::LightBookInputFactory)
