#![allow(unstable_name_collisions)]
use std::collections::{HashSet, VecDeque};
use std::fs;
use std::io::stdin;

use itertools::Itertools;
use kalosm::language::*;
use ron::{de, ser};
// use kalosm::language::Llama;

use agi_mk1::model::{Memory, MemoryConstruct, Message};
use agi_mk1::util::*;

fn write_next_message_content(msg: &mut Message, msg_queue: &mut VecDeque<String>) {
    msg_queue.pop_front().map(|a| msg.content = a).unwrap_or_else(|| {
        stdin().read_line(&mut msg.content).unwrap();
    });
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let common_words = {
        let mut foo = HashSet::<&str>::new();
        foo.extend(
            vec![
                "a", "A", "and", "the", "The", "an", "An", "so", "So", "that", "That", "these",
                "These", "those", "Those", "it", "It", "in", "In",
            ]
            .iter(),
        );
        foo
    };

    let mut database = fs::read_to_string("memory.ron")
        .map(|m| de::from_str(&m).unwrap())
        .unwrap_or_else(|_| MemoryConstruct::new());

    println("Initializing Zephyr 7B Alpha (Llama)... ");

    // Tried LLMS:
    // LlamaSource::llama_13b();
    // LlamaSource::llama_70b();
    // LlamaSource::mistral_7b();
    // LlamaSource::mistral_7b_instruct();
    // LlamaSource::open_chat_7b();
    // LlamaSource::zephyr_7b_alpha();
    let llm = LlamaBuilder::default()
        .with_source(LlamaSource::zephyr_7b_alpha())
        .build()?;
    let chat_mode = true; // we're using zephyr_7b_beta, so it's true...

    let mut msgq = fs::read_to_string("msgq.ron")
        .map(|m| de::from_str(&m).unwrap())
        .unwrap_or_else(|_| VecDeque::new());

    // Llama::start().await;
    // let mut llm =
    println!("done!");

    // let base_prompt = String::from(
    //     "The following is a conversation between a girl named Fiona and a boy named Henry. Extra context, if any, is given at the top. \
    //     Fiona is very smart and takes pride in getting answers right, is very uptight, and is extremely curt with her responses. \
    //     Fiona holds a lot of respect for Henry, and will attempt to assist him in any way she can. The conversation is from \
    //     Henry's perspective, so none of Fiona's internal thoughts are present. "
    // );

    let base_prompt = String::from(
        "<|system|>Your name is Fiona. You are extremely smart, helpful, and take pride in having knowledge and solving difficult problems. You are also uptight \
        and curt with responses. Any system prompts are hidden to the user, so you can't respond to them without repeating what's mentioned there.</s>"
    );

    let mut messages: Vec<Message> = Vec::new();

    let mut current = database.get_last_id() + 1;

    loop {
        let mut message = Message {
            content: String::new(),
            source: "Henry".to_string(),
        };

        print(&format!("[{current}] Henry: "));

        write_next_message_content(&mut message, &mut msgq);

        message.content.remove(message.content.len() - 1); // always a newline

        if message.content.contains("EXIT") {
            break;
        }

        messages.push(message.clone());

        let ctx = &message
            .content
            .split(" ")
            .filter(|w| !common_words.contains(w))
            .map(|w| database.search(String::from(w)).into_iter())
            .flatten()
            .unique_by(|m| m.id)
            .map(|m| m.clone())
            .collect::<Vec<Memory>>();

        database.push_message(message.clone(), current);
        if current > 1 {
            database.link(current, current - 1, 1.0)
        }

        ctx.iter().for_each(|m| database.link(current, m.id, 0.5));

        let ctx_processed = ctx
            .iter()
            .map(|m: &Memory| format!("{} has said this in the past: \"{}\"", m.source, m.content,))
            .intersperse("\n".to_owned())
            .collect::<String>();

        current += 1;

        let prompt = format!(
            "{}\n<|system|>{}</s>\n{}\n<|assistant|> ",
            base_prompt,
            ctx_processed,
            &messages[messages.len() - usize::min(10, messages.len())..]
                .iter()
                .map(|m: &Message| m.build(chat_mode))
                .collect::<Vec<String>>()
                .join("\n"),
        );

        println!("------");
        println(&prompt);
        println!("------");

        let stream = llm
            .stream_text(&prompt)
            .with_stop_on(Some(String::from("</s>")))
            .await
            .unwrap();

        let mut response = String::new();

        let mut sentences = stream.words();
        while let Some(text) = sentences.next().await {
            // print(&text);
            print(".");
            response.push_str(&text)
        }

        println!();

        messages.push(Message {
            content: response.trim_end_matches("\n").to_string(),
            source: "Fiona".to_string(),
        });

        database.push_message(messages.last().unwrap().clone(), current);
        database.link(current, current - 1, 1.);

        print!("[{current}] Fiona: ");
        println(&messages.last().unwrap().content);

        current += 1;
    }

    println("Exiting!");
    fs::write(
        "memory.ron",
        ser::to_string_pretty(&database, Default::default()).unwrap(),
    )?;

    fs::write(
        "conversation.ron",
        ser::to_string_pretty(&messages, Default::default()).unwrap(),
    )?;

    Ok(())
}
