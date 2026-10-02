use mox_platform_gateway_svc::mailer::{EmailMessage, SmtpClient, SmtpConfig};

fn message() -> EmailMessage {
    EmailMessage {
        message_id: "validation-only".into(),
        to: vec!["recipient@example.com".into()],
        cc: None,
        bcc: None,
        subject: "validation".into(),
        text_body: Some("body".into()),
        html_body: None,
        attachments: None,
        headers: None,
        priority: None,
    }
}

#[test]
fn smtp_placeholder_configuration_never_reports_delivery() {
    assert!(SmtpClient::new(SmtpConfig::default()).send(&message()).is_err());
}

#[test]
fn missing_smtp_credentials_never_report_delivery() {
    let config = SmtpConfig { host: "localhost".into(), ..SmtpConfig::default() };
    assert!(SmtpClient::new(config).send(&message()).is_err());
}

#[test]
fn real_connection_failure_is_not_delivery_success() {
    // A real OS-assigned port is reserved but not listening; no fake SMTP server.
    let socket = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = socket.local_addr().unwrap().port();
    drop(socket);
    let config = SmtpConfig {
        host: "127.0.0.1".into(),
        port,
        username: "validation-user".into(),
        password: "validation-password".into(),
        timeout_seconds: 1,
        ..SmtpConfig::default()
    };
    assert!(SmtpClient::new(config).send(&message()).is_err());
}
