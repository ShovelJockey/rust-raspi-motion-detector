use mail_send::{mail_builder, SmtpClientBuilder};

fn create_message<'a>(time: String) -> mail_builder::MessageBuilder<'a> {
    let origin = ("Motion Detector", "jamie.hirst10@gmail.com");
    let target = ("Jamie Hirst", "jamie.hirst10@gmail.com");
    return mail_builder::MessageBuilder::new()
        .from(origin)
        .to(target)
        .subject("Motion Detected")
        .text_body(format!("Motion Detected at {time}, recording incident"));
}

async fn send_mail(time: String) {
    let smtp_client_builder = SmtpClientBuilder::new("smtp.gmail.com", 465);
    let mut smtp_client = smtp_client_builder
        .implicit_tls(false)
        .credentials(("username", "password"))
        .connect()
        .await
        .unwrap();
    let message = create_message(time);
    smtp_client.send(message).await.unwrap();
}
