//! Integração com um RabbitMQ real: consumo, roteamento por
//! `event_type`, payload malformado, falha de handler e recuperação de
//! offset (§33).
//!
//! Semântica coberta aqui: **at-least-once** — o offset é gravado
//! depois do handler, então handler que falha NÃO avança o offset e o
//! evento volta no próximo (re)início.

mod common;

use std::time::Duration;

use common::{
    AlwaysFailHandler, CollectHandler, FailThenCollectHandler, TestBroker, client_side_strategy,
    envelope, expect_no_delivery, matriculado, next_with_timeout, unique,
};
use gerax_rabbitmq_stream::{
    ConsumerConfig, JsonEventSerializer, PublishMode, ReconnectPolicy, StartOffset,
};
use tokio::sync::mpsc;

const TIMEOUT: Duration = Duration::from_secs(10);

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn round_trip_publica_e_consome_o_evento() {
    let Some(broker) = TestBroker::open("cons-roundtrip").await else {
        return;
    };

    let (tx, mut rx) = mpsc::unbounded_channel();
    let consumer = broker
        .connection
        .consumer(broker.consumer_config("c-roundtrip"), JsonEventSerializer)
        .await
        .expect("consumer");
    consumer.register::<common::AlunoMatriculado, _>("aluno.matriculado", CollectHandler::new(tx));
    let subscription = consumer.subscribe().await.expect("subscription");

    let producer = broker.producer().await;
    let published = envelope(matriculado("a-1"));
    let event_id = published.event_id.to_string();
    let event_type = published.event_type.clone();
    producer.publish(&published).await.expect("publicado");
    producer.close().await.expect("producer fechado");

    let received = next_with_timeout(&mut rx, TIMEOUT).await;
    assert_eq!(received.event.aluno_id, "a-1");
    assert_eq!(received.event.curso, "CMF");
    assert_eq!(received.metadata.event_type, event_type);
    assert_eq!(
        received.id.as_uuid().to_string(),
        event_id,
        "identidade preservada"
    );
    assert_eq!(
        received.metadata.occurred_at.timestamp(),
        published.occurred_at.timestamp()
    );

    subscription.stop().await.expect("subscription parada");
    broker.cleanup().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn multiplos_eventos_sao_consumidos_em_ordem() {
    let Some(broker) = TestBroker::open("cons-multi").await else {
        return;
    };

    let producer = broker.producer().await;
    let batch: Vec<_> = (0..20)
        .map(|index| envelope(matriculado(&format!("a-{index}"))))
        .collect();
    producer
        .publish_batch(&batch)
        .await
        .expect("lote publicado");
    producer.close().await.expect("producer fechado");

    let (tx, mut rx) = mpsc::unbounded_channel();
    let consumer = broker
        .connection
        .consumer(broker.consumer_config("c-multi"), JsonEventSerializer)
        .await
        .expect("consumer");
    consumer.register::<common::AlunoMatriculado, _>("aluno.matriculado", CollectHandler::new(tx));
    let subscription = consumer.subscribe().await.expect("subscription");

    for index in 0..20 {
        let received = next_with_timeout(&mut rx, TIMEOUT).await;
        assert_eq!(received.event.aluno_id, format!("a-{index}"));
    }

    subscription.stop().await.expect("subscription parada");
    broker.cleanup().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dois_consumidores_do_mesmo_stream_recebem_cada_um_sua_copia() {
    let Some(broker) = TestBroker::open("cons-broadcast").await else {
        return;
    };

    let (tx_a, mut rx_a) = mpsc::unbounded_channel();
    let (tx_b, mut rx_b) = mpsc::unbounded_channel();

    let consumer_a = broker
        .connection
        .consumer(broker.consumer_config("c-broadcast-a"), JsonEventSerializer)
        .await
        .expect("consumer a");
    consumer_a
        .register::<common::AlunoMatriculado, _>("aluno.matriculado", CollectHandler::new(tx_a));
    let subscription_a = consumer_a.subscribe().await.expect("subscription a");

    let consumer_b = broker
        .connection
        .consumer(broker.consumer_config("c-broadcast-b"), JsonEventSerializer)
        .await
        .expect("consumer b");
    consumer_b
        .register::<common::AlunoMatriculado, _>("aluno.matriculado", CollectHandler::new(tx_b));
    let subscription_b = consumer_b.subscribe().await.expect("subscription b");

    let producer = broker.producer().await;
    producer
        .publish(&envelope(matriculado("a-1")))
        .await
        .expect("publicado");
    producer.close().await.expect("producer fechado");

    assert_eq!(
        next_with_timeout(&mut rx_a, TIMEOUT).await.event.aluno_id,
        "a-1"
    );
    assert_eq!(
        next_with_timeout(&mut rx_b, TIMEOUT).await.event.aluno_id,
        "a-1"
    );

    subscription_a.stop().await.expect("subscription a parada");
    subscription_b.stop().await.expect("subscription b parada");
    broker.cleanup().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn evento_sem_handler_e_ignorado_sem_travar_o_consumidor() {
    let Some(broker) = TestBroker::open("cons-unknown").await else {
        return;
    };

    let (tx, mut rx) = mpsc::unbounded_channel();
    let consumer = broker
        .connection
        .consumer(broker.consumer_config("c-unknown"), JsonEventSerializer)
        .await
        .expect("consumer");
    consumer.register::<common::AlunoMatriculado, _>("aluno.matriculado", CollectHandler::new(tx));
    let subscription = consumer.subscribe().await.expect("subscription");

    let producer = broker.producer().await;
    // Evento de outro contexto + evento do contexto registrado.
    producer
        .publish(&envelope(common::TurmaCriada {
            turma_id: "t-1".into(),
        }))
        .await
        .expect("publicado (sem handler)");
    producer
        .publish(&envelope(matriculado("a-1")))
        .await
        .expect("publicado");
    producer.close().await.expect("producer fechado");

    let received = next_with_timeout(&mut rx, TIMEOUT).await;
    assert_eq!(
        received.event.aluno_id, "a-1",
        "o evento registrado não pode se perder"
    );

    subscription.stop().await.expect("subscription parada");
    broker.cleanup().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn payload_malformado_e_pulado_e_o_stream_segue() {
    let Some(broker) = TestBroker::open("cons-malformed").await else {
        return;
    };

    let (tx, mut rx) = mpsc::unbounded_channel();
    let consumer = broker
        .connection
        .consumer(broker.consumer_config("c-malformed"), JsonEventSerializer)
        .await
        .expect("consumer");
    consumer.register::<common::AlunoMatriculado, _>("aluno.matriculado", CollectHandler::new(tx));
    let subscription = consumer.subscribe().await.expect("subscription");

    // JSON inválido no body, sem application_properties.
    broker.publish_raw(b"isso-nao-e-um-json").await;
    // Properties válidas, payload de outro tipo.
    broker
        .publish_raw(br#"{"aluno_id": 123456789012345678901234567890}"#)
        .await;

    let producer = broker.producer().await;
    producer
        .publish(&envelope(matriculado("a-1")))
        .await
        .expect("publicado");
    producer.close().await.expect("producer fechado");

    let received = next_with_timeout(&mut rx, TIMEOUT).await;
    assert_eq!(received.event.aluno_id, "a-1");

    subscription.stop().await.expect("subscription parada");
    broker.cleanup().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn handler_que_falha_nao_avanca_o_offset() {
    let Some(broker) = TestBroker::open("cons-handler-fail").await else {
        return;
    };

    let (strategy, store) = client_side_strategy();
    let (failing, calls) = AlwaysFailHandler::new();

    let consumer = broker
        .connection
        .consumer(broker.consumer_config("c-fail"), JsonEventSerializer)
        .await
        .expect("consumer")
        .with_offset_strategy(strategy);
    consumer.register::<common::AlunoMatriculado, _>("aluno.matriculado", failing);
    let subscription = consumer.subscribe().await.expect("subscription");

    let producer = broker.producer().await;
    producer
        .publish(&envelope(matriculado("a-1")))
        .await
        .expect("publicado");
    producer.close().await.expect("producer fechado");

    // O handler roda e falha: nenhum offset é gravado.
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);
    assert!(store.is_empty(), "nenhum offset deve ter sido gravado");

    subscription.stop().await.expect("subscription parada");

    // Consumer novo com o MESMO offset store: o evento volta
    // (at-least-once).
    let (tx, mut rx) = mpsc::unbounded_channel();
    let consumer = broker
        .connection
        .consumer(broker.consumer_config("c-fail"), JsonEventSerializer)
        .await
        .expect("consumer")
        .with_offset_strategy(common::strategy_for(&store));
    consumer.register::<common::AlunoMatriculado, _>("aluno.matriculado", CollectHandler::new(tx));
    let subscription = consumer.subscribe().await.expect("resubscription");

    let received = next_with_timeout(&mut rx, TIMEOUT).await;
    assert_eq!(
        received.event.aluno_id, "a-1",
        "evento deve ser redelivered"
    );

    subscription.stop().await.expect("subscription parada");
    assert_eq!(store.len(), 1, "offset gravado após o handler bem-sucedido");
    broker.cleanup().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn offset_client_side_e_recuperado_apos_restart() {
    let Some(broker) = TestBroker::open("cons-offset-client").await else {
        return;
    };

    let (strategy, store) = client_side_strategy();

    // Primeira sessão: consome o que já existe e para. O offset exato
    // gravado depende de quantos eventos o broker entregou no lote,
    // então a asserção é sobre a retomada, não sobre um id fixo.
    let (tx, mut rx) = mpsc::unbounded_channel();
    let consumer = broker
        .connection
        .consumer(broker.consumer_config("c-restart"), JsonEventSerializer)
        .await
        .expect("consumer")
        .with_offset_strategy(strategy);
    consumer.register::<common::AlunoMatriculado, _>("aluno.matriculado", CollectHandler::new(tx));
    let subscription = consumer.subscribe().await.expect("subscription");

    let producer = broker.producer().await;
    for index in 0..3 {
        producer
            .publish(&envelope(matriculado(&format!("a-{index}"))))
            .await
            .expect("publicado");
    }

    let first = next_with_timeout(&mut rx, TIMEOUT).await;
    subscription.stop().await.expect("subscription parada");
    assert_eq!(store.len(), 1, "um offset client-side gravado");
    drop(rx);

    // Segunda sessão com o MESMO store: retoma depois do último offset,
    // sem reprocessar o que já foi tratado.
    let (tx, mut rx) = mpsc::unbounded_channel();
    let consumer = broker
        .connection
        .consumer(broker.consumer_config("c-restart"), JsonEventSerializer)
        .await
        .expect("consumer")
        .with_offset_strategy(common::strategy_for(&store));
    consumer.register::<common::AlunoMatriculado, _>("aluno.matriculado", CollectHandler::new(tx));
    let subscription = consumer.subscribe().await.expect("resubscription");

    // Publica depois da retomada: garante uma entrega mesmo se a
    // primeira sessão já tiver consumido o stream inteiro.
    producer
        .publish(&envelope(matriculado("a-novo")))
        .await
        .expect("publicado");
    producer.close().await.expect("producer fechado");

    let mut delivered = vec![next_with_timeout(&mut rx, TIMEOUT).await];
    while delivered.last().expect("primeira entrega").event.aluno_id != "a-novo" {
        delivered.push(next_with_timeout(&mut rx, TIMEOUT).await);
    }

    assert!(
        delivered
            .iter()
            .all(|event| event.event.aluno_id != first.event.aluno_id),
        "evento já processado foi reprocessado: {:?}",
        delivered
            .iter()
            .map(|event| event.event.aluno_id.as_str())
            .collect::<Vec<_>>(),
    );

    subscription.stop().await.expect("subscription parada");
    broker.cleanup().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn offset_server_side_e_persistido_no_broker() {
    let Some(broker) = TestBroker::open("cons-offset-server").await else {
        return;
    };

    let consumer_name = unique("c-server");
    let config = broker.consumer_config(&consumer_name);

    let producer = broker.producer().await;
    producer
        .publish(&envelope(matriculado("a-1")))
        .await
        .expect("publicado");
    producer.close().await.expect("producer fechado");

    // Primeira sessão (ServerSide é o padrão): consome e grava o offset
    // no broker.
    let (tx, mut rx) = mpsc::unbounded_channel();
    let consumer = broker
        .connection
        .consumer(config.clone(), JsonEventSerializer)
        .await
        .expect("consumer");
    consumer.register::<common::AlunoMatriculado, _>("aluno.matriculado", CollectHandler::new(tx));
    let subscription = consumer.subscribe().await.expect("subscription");
    assert_eq!(
        next_with_timeout(&mut rx, TIMEOUT).await.event.aluno_id,
        "a-1"
    );
    subscription.stop().await.expect("subscription parada");

    // Segunda sessão com o MESMO consumer_name: o StartOffset::First do
    // config é ignorado porque o broker já tem offset gravado.
    let (tx, mut rx) = mpsc::unbounded_channel();
    let consumer = broker
        .connection
        .consumer(config, JsonEventSerializer)
        .await
        .expect("consumer");
    consumer.register::<common::AlunoMatriculado, _>("aluno.matriculado", CollectHandler::new(tx));
    let subscription = consumer.subscribe().await.expect("resubscription");

    expect_no_delivery(&mut rx, Duration::from_secs(2)).await;

    // Um evento novo ainda chega: o consumidor continua no mesmo offset.
    let producer = broker.producer().await;
    producer
        .publish(&envelope(matriculado("a-2")))
        .await
        .expect("publicado");
    producer.close().await.expect("producer fechado");
    assert_eq!(
        next_with_timeout(&mut rx, TIMEOUT).await.event.aluno_id,
        "a-2"
    );

    subscription.stop().await.expect("subscription parada");
    broker.cleanup().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn restart_com_consumer_novo_relê_do_inicio() {
    let Some(broker) = TestBroker::open("cons-restart-first").await else {
        return;
    };

    let config = gerax_rabbitmq_stream::ConsumerConfig::builder()
        .stream(&broker.stream)
        .consumer_name(unique("c-first"))
        .offset(StartOffset::First)
        .build()
        .expect("config valida");

    let producer = broker.producer().await;
    producer
        .publish(&envelope(matriculado("a-1")))
        .await
        .expect("publicado");
    producer.close().await.expect("producer fechado");

    let (tx, mut rx) = mpsc::unbounded_channel();
    let consumer = broker
        .connection
        .consumer(config, JsonEventSerializer)
        .await
        .expect("consumer");
    consumer.register::<common::AlunoMatriculado, _>("aluno.matriculado", CollectHandler::new(tx));
    let subscription = consumer.subscribe().await.expect("subscription");

    assert_eq!(
        next_with_timeout(&mut rx, TIMEOUT).await.event.aluno_id,
        "a-1"
    );
    subscription.stop().await.expect("subscription parada");
    broker.cleanup().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn handler_com_retry_recebe_o_evento_na_mesma_sessao_e_no_restart() {
    let Some(broker) = TestBroker::open("cons-retry").await else {
        return;
    };

    let (strategy, store) = client_side_strategy();
    let (handler, mut rx) = FailThenCollectHandler::new(1);

    let consumer = broker
        .connection
        .consumer(broker.consumer_config("c-retry"), JsonEventSerializer)
        .await
        .expect("consumer")
        .with_offset_strategy(strategy);
    consumer.register::<common::AlunoMatriculado, _>("aluno.matriculado", handler);
    let subscription = consumer.subscribe().await.expect("subscription");

    let producer = broker.producer().await;
    producer
        .publish(&envelope(matriculado("a-1")))
        .await
        .expect("publicado");
    producer.close().await.expect("producer fechado");

    // A primeira tentativa falha: o offset não avança e o adapter não
    // reentrega na mesma sessão (redelivery vem no restart).
    tokio::time::sleep(Duration::from_millis(500)).await;
    expect_no_delivery(&mut rx, Duration::from_millis(300)).await;
    assert!(store.is_empty());

    subscription.stop().await.expect("subscription parada");

    let (tx, mut rx) = mpsc::unbounded_channel();
    let consumer = broker
        .connection
        .consumer(broker.consumer_config("c-retry"), JsonEventSerializer)
        .await
        .expect("consumer")
        .with_offset_strategy(common::strategy_for(&store));
    consumer.register::<common::AlunoMatriculado, _>("aluno.matriculado", CollectHandler::new(tx));
    let subscription = consumer.subscribe().await.expect("resubscription");

    let received = next_with_timeout(&mut rx, TIMEOUT).await;
    assert_eq!(received.event.aluno_id, "a-1");

    subscription.stop().await.expect("subscription parada");
    broker.cleanup().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "reinicia o broker: defina RABBITMQ_STREAM_RESTART_CMD para rodar"]
async fn reconnect_apos_o_broker_reiniciar() {
    let Some(config) = common::gate_broker() else {
        return;
    };

    let Some(restart) = std::env::var("RABBITMQ_STREAM_RESTART_CMD").ok() else {
        eprintln!("skipped: set RABBITMQ_STREAM_RESTART_CMD to run the reconnect test");
        return;
    };

    let broker = TestBroker::connect(config, "cons-broker-restart").await;

    // Backoff curto e ilimitado: o broker pode ficar alguns segundos
    // fora do ar.
    let config = ConsumerConfig::builder()
        .stream(&broker.stream)
        .consumer_name(unique("c-restart-broker"))
        .offset(StartOffset::First)
        .reconnect(ReconnectPolicy::new(
            Duration::from_millis(500),
            1.0,
            Duration::from_millis(500),
            0,
        ))
        .build()
        .expect("config valida");

    let (tx, mut rx) = mpsc::unbounded_channel();
    let consumer = broker
        .connection
        .consumer(config, JsonEventSerializer)
        .await
        .expect("consumer");
    consumer.register::<common::AlunoMatriculado, _>("aluno.matriculado", CollectHandler::new(tx));
    let subscription = consumer.subscribe().await.expect("subscription");

    let producer = broker.producer().await;
    producer
        .publish(&envelope(matriculado("a-1")))
        .await
        .expect("publicado");
    producer.close().await.expect("producer fechado");
    assert_eq!(
        next_with_timeout(&mut rx, TIMEOUT).await.event.aluno_id,
        "a-1"
    );

    // Derruba e levanta o broker: a conexão do consumer cai.
    let status = std::process::Command::new("sh")
        .arg("-c")
        .arg(&restart)
        .status()
        .expect("comando de restart executado");
    assert!(status.success(), "restart do broker falhou: {status}");

    // O broker volta; a assinatura reassina sozinha (backoff) e volta a
    // consumir.
    let producer = wait_for_producer(&broker).await;
    producer
        .publish(&envelope(matriculado("a-2")))
        .await
        .expect("publicado após restart");
    producer.close().await.expect("producer fechado");

    let received = next_with_timeout(&mut rx, Duration::from_secs(60)).await;
    assert_eq!(received.event.aluno_id, "a-2", "consumer não reassinou");

    subscription.stop().await.expect("subscription parada");
    broker.cleanup().await;
}

/// Espera o broker voltar a aceitar produção (o restart é assíncrono).
async fn wait_for_producer(broker: &TestBroker) -> gerax_rabbitmq_stream::RabbitMqStreamProducer {
    let mut last_error = None;
    for _ in 0..60 {
        match broker
            .connection
            .producer(PublishMode::Confirmed, 100, JsonEventSerializer)
            .await
        {
            Ok(producer) => return producer,
            Err(source) => {
                last_error = Some(source);
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
    }
    panic!(
        "broker não voltou após o restart (stream {}): {last_error:?}",
        broker.stream
    );
}
