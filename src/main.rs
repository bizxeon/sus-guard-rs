use std::borrow::BorrowMut;
use std::{fs, env};

use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::model::prelude::{ChannelId, MessageId, GuildId, Member, MessageUpdateEvent, GuildDeleteEvent};
use serenity::prelude::*;
use serenity::framework::standard::StandardFramework;

struct Handler;

enum SusMessageSate {
    Deleted,
    Edited,
}

struct SusMessageObject {
    state: SusMessageSate,
    content: String,
    username: String,
    avatar_url: String,
    channel_id: ChannelId,
}

const MESSAGES_BUFFER_LIMIT: usize = 256;
const SUS_MESSAGES_LIMIT: usize = 256;

struct MessageBufferType(Vec<(Option<Member>, Message)>);
struct SusMessagesType(Vec<SusMessageObject>);

impl MessageBufferType {
    fn new() -> MessageBufferType {
        MessageBufferType(Vec::new())
    }

    fn data(&self) -> &Vec<(Option<Member>, Message)> {
        &self.0
    }

    fn data_mut(&mut self) -> &mut Vec<(Option<Member>, Message)> {
        &mut self.0
    }

    fn overflow_prevent(&mut self) {
        if self.0.len() > MESSAGES_BUFFER_LIMIT {
            let _ = self.0.pop();
        }        
    }
}

impl SusMessagesType {
    fn new() -> SusMessagesType {
        SusMessagesType(Vec::new())
    }

    fn data(&self) -> &Vec<SusMessageObject> {
        &self.0
    }

    fn data_mut(&mut self) -> &mut Vec<SusMessageObject> {
        &mut self.0
    }

    fn overflow_prevent(&mut self) {
        if self.0.len() > SUS_MESSAGES_LIMIT {
            let _ = self.0.pop();
        }        
    }
}

lazy_static::lazy_static! {
    // static ref MESSAGES_BUFFER: Mutex<Vec<(Option<Member>, Message)>> = Mutex::new(Vec::new());
    static ref MESSAGES_BUFFER: Mutex<MessageBufferType> = Mutex::new(MessageBufferType::new());
    static ref SUS_MESSAGES: Mutex<SusMessagesType> = Mutex::new(SusMessagesType::new());
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, _ctx: Context, _new_message: Message) {
        let mut _member: Option<Member> = None;
        let guilds = &_ctx.cache.guilds(); // Replace with your server's ID

        for guild in guilds {
            let member_search = guild.member(&_ctx.http, _new_message.author.id).await;

            match member_search {
                Ok(value) => { _member = Some(value) },
                Err(_) => {}
            }
        }

        let mut message_block = MESSAGES_BUFFER.lock().await;

        message_block.overflow_prevent();
        message_block.data_mut().push((_member, _new_message.clone()));

        if &_new_message.content.trim_end().to_string() == &".snipe".to_string() {
            let mut idx: usize = 0;
            let mut sus_message_buffer = SUS_MESSAGES.lock().await;

            for sus_message in sus_message_buffer.data_mut() {
                if sus_message.channel_id == _new_message.channel_id {
                    match _ctx.cache.guild_channel(_new_message.channel_id) {
                        Some(channel) => {
                            match sus_message.state {
                                SusMessageSate::Deleted => {
                                    let _ = channel. send_message(&_ctx, |m| {
                                        m.embed(|e| e.title("Deleted message!".to_string())
                                            .author(|a| a 
                                                .name(&sus_message.username)
                                                .icon_url(&sus_message.avatar_url)
                                            )
                                            .description(&sus_message.content.clone())
                                        )
                                    }).await;
                                },
                                SusMessageSate::Edited => {
                                    let _ = channel.send_message(&_ctx, |m| {
                                        m.embed(|e| e.title("Edited message!".to_string())
                                            .author(|a| a 
                                                .name(&sus_message.username)
                                                .icon_url(&sus_message.avatar_url)
                                            )
                                            .description(&sus_message.content)
                                        )
                                    }).await;
                                }
                            }
                        },
                        None => {}
                    }

                    sus_message_buffer.data_mut().remove(idx);
                    break;
                }

                idx = idx + 1;
            }
        }
    }

    async fn message_delete(&self, _ctx: Context, _channel_id: ChannelId, _deleted_message_id: MessageId, _guild_id: Option<GuildId>) {
        let messages_block = MESSAGES_BUFFER.lock().await;

        for message_block in messages_block.data().iter() {
            let message = &message_block.1;
    
            if message.id == _deleted_message_id {
                match &message_block.0 {
                    Some(member) => {
                        let mut buffer = SUS_MESSAGES.lock().await;

                        buffer.overflow_prevent();
                        buffer.data_mut().push(SusMessageObject {
                            state: SusMessageSate::Deleted,
                            content: message.content.clone(),
                            username: member.distinct(),
                            avatar_url: member.face(),
                            channel_id: message.channel_id
                        });
                    },
                    None => {
                        let mut buffer = SUS_MESSAGES.lock().await;

                        buffer.overflow_prevent();
                        buffer.data_mut().push(SusMessageObject {
                            state: SusMessageSate::Deleted,
                            content: message.content.clone(),
                            username: message.author.name.clone(),
                            avatar_url: message.author.face(),
                            channel_id: message.channel_id
                        });
                    }
                }

                break;
            }
        }
    }

    async fn message_update(&self, _ctx: Context, _old_if_available: Option<Message>, _new: Option<Message>, _event: MessageUpdateEvent) {
        let mut messages_block = MESSAGES_BUFFER.lock().await;

        for message_block in messages_block.data_mut().iter_mut() {
            if message_block.1.id == _event.id {
                let message = message_block.1.borrow_mut();

                match _event.content {
                    Some(new_message_content) => {
                        match &message_block.0 {
                            Some(member) => {
                                let mut buffer = SUS_MESSAGES.lock().await;

                                buffer.overflow_prevent();
                                buffer.data_mut().push(SusMessageObject {
                                    state: SusMessageSate::Edited,
                                    content: message.content.clone(),
                                    username: member.distinct(),
                                    avatar_url: member.face(),
                                    channel_id: message.channel_id
                                });
                            },
                            None => {
                                let mut buffer = SUS_MESSAGES.lock().await;

                                buffer.overflow_prevent();
                                buffer.data_mut().push(SusMessageObject {
                                    state: SusMessageSate::Edited,
                                    content: message.content.clone(),
                                    username: message.author.name.clone(),
                                    avatar_url: message.author.face(),
                                    channel_id: message.channel_id
                                });
                            }
                        }  

                        message.content = new_message_content.to_owned();
                    },
                    None => {}
                }

                break;
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let framework = StandardFramework::new();

    let args = env::args().collect::<Vec<String>>();
    let token = args.get(1).unwrap_or(&"--".to_string()).to_owned();
    
    let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT | GatewayIntents::GUILD_MESSAGES | GatewayIntents::GUILD_MEMBERS | GatewayIntents::GUILDS;
    let mut client = Client::builder(token, intents)
        .event_handler(Handler)
        .framework(framework)
        .await
        .expect("Error creating client");

    // start listening for events by starting a single shard
    if let Err(why) = client.start().await {
        println!("An error occurred while running the client: {:?}", why);
    }
}
