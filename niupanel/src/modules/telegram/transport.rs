use niupanel_common::error::Result;
use niupanel_plugin::{PluginInvokeRequest, PluginInvokeResponse};
use tokio::sync::Mutex;

// A transport has one long-lived worker. Connection tests share it with ticks.
static COMMAND: Mutex<()> = Mutex::const_new(());

pub(crate) async fn invoke(
    plugin_id: &str,
    request: PluginInvokeRequest,
) -> Result<PluginInvokeResponse> {
    let _command = COMMAND.lock().await;
    crate::modules::plugins::service::unified_plugin_service()
        .invoke_telegram_channel(plugin_id, request)
        .await
}
