// SPDX-License-Identifier: GPL-3.0-only

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() -> cosmic::iced::Result {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                // While the popup redraws, iced warns of each event a
                // subscription doesn't take: dozens a second.
                .unwrap_or_else(|_| {
                    "warn,iced_futures::subscription::tracker=error,cosmic_applet_saisei=info"
                        .into()
                }),
        )
        .init();
    let _ = tracing_log::LogTracer::init();

    tracing::info!("Starting Saisei applet with version {VERSION}");

    cosmic_applet_saisei::run()
}
