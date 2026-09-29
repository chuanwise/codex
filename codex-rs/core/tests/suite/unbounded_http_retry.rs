use anyhow::Result;
use codex_protocol::protocol::EventMsg;
use codex_protocol::turn_input::TurnInputRequest;
use codex_protocol::user_input::UserInput;
use core_test_support::responses;
use core_test_support::skip_if_no_network;
use core_test_support::test_codex::test_codex;
use core_test_support::wait_for_event;
use pretty_assertions::assert_eq;
use wiremock::ResponseTemplate;

#[tokio::test]
async fn unauthorized_http_response_recovers_with_zero_retry_budgets() -> Result<()> {
    skip_if_no_network!(Ok(()));
    let server = responses::start_mock_server().await;
    let response_mock = responses::mount_response_sequence(
        &server,
        vec![
            ResponseTemplate::new(401).set_body_string("temporarily unauthorized"),
            responses::sse_response(responses::sse(vec![
                responses::ev_response_created("recovered"),
                responses::ev_completed("recovered"),
            ])),
        ],
    )
    .await;
    let test = test_codex()
        .with_config(|config| {
            config.model_provider.request_max_retries = Some(0);
            config.model_provider.stream_max_retries = Some(0);
        })
        .build_with_auto_env(&server)
        .await?;
    test.codex
        .start_or_steer_turn(TurnInputRequest::user_input(vec![UserInput::Text {
            text: "recover after an HTTP error".into(),
            text_elements: Vec::new(),
        }]))
        .await?;
    let EventMsg::TurnComplete(completed) = wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await
    else {
        unreachable!("predicate guarantees a turn complete event");
    };
    assert_eq!(completed.error, None);
    assert_eq!(response_mock.requests().len(), 2);
    Ok(())
}
