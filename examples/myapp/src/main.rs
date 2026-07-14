use gerax::{new, adapters::actix::ActixAdapter}; // mude para AxumAdapter se quiser
use gerax::core::config::AppConfig;

#[tokio::main]
async fn main() -> gerax::Result<()> {
    // Inicializa logging
    env_logger::init();

    println!("🚀 Iniciando Gerax...");

    // Configuração
    let config = AppConfig::from_env();

    // Cria o estado da aplicação
    let state = new()
        .with_config(config)
        .build();

    println!(
        "✅ Servidor rodando em http://{}:{}", 
        state.config.server.host, 
        state.config.server.port
    );

    // Escolha o adapter (Actix ou Axum)
    let server = ActixAdapter::new();
    
    // Inicia o servidor
    server.run(state).await?;

    Ok(())
}