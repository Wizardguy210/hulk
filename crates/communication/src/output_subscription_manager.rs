use std::collections::{hash_map::Entry, HashMap};

use log::{error, info};
use tokio::{
    spawn,
    sync::{mpsc, oneshot},
};
use uuid::Uuid;

use crate::{
    id_tracker::{self, get_message_id},
    requester, responder,
    types::SubscribedOutput,
    OutputHierarchy, SubscriberMessage,
};

use super::{Cycler, CyclerOutput};

#[derive(Debug)]
pub enum Message {
    Connect {
        requester: mpsc::Sender<requester::Message>,
    },
    Disconnect,
    Subscribe {
        output: CyclerOutput,
        subscriber: mpsc::Sender<SubscriberMessage>,
        response_sender: oneshot::Sender<Uuid>,
    },
    Unsubscribe {
        output: CyclerOutput,
        uuid: Uuid,
    },
    Update {
        cycler: Cycler,
        outputs: Vec<SubscribedOutput>,
    },
    UpdateOutputHierarchy {
        hierarchy: OutputHierarchy,
    },
    GetOutputHierarchy {
        response_sender: oneshot::Sender<Option<OutputHierarchy>>,
    },
}

pub async fn output_subscription_manager(
    mut receiver: mpsc::Receiver<Message>,
    sender: mpsc::Sender<Message>,
    id_tracker: mpsc::Sender<id_tracker::Message>,
    responder: mpsc::Sender<responder::Message>,
) {
    let mut subscribed_outputs: HashMap<
        CyclerOutput,
        HashMap<Uuid, mpsc::Sender<SubscriberMessage>>,
    > = HashMap::new();
    let mut requester = None;
    let mut hierarchy = None;
    while let Some(message) = receiver.recv().await {
        match message {
            Message::Connect {
                requester: new_requester,
            } => {
                for (output, subscribers) in &subscribed_outputs {
                    let subscribers = subscribers.values().cloned().collect();
                    subscribe(
                        output.clone(),
                        subscribers,
                        &id_tracker,
                        &responder,
                        &new_requester,
                    )
                    .await
                }
                query_output_hierarchy(sender.clone(), &id_tracker, &responder, &new_requester)
                    .await;
                requester = Some(new_requester);
            }
            Message::Disconnect => {
                requester = None;
            }
            Message::Subscribe {
                output,
                subscriber: output_sender,
                response_sender,
            } => {
                let uuid = Uuid::new_v4();
                response_sender.send(uuid).unwrap();
                add_subscription(
                    &mut subscribed_outputs,
                    uuid,
                    output,
                    output_sender,
                    &id_tracker,
                    &responder,
                    &requester,
                )
                .await;
            }
            Message::Unsubscribe { output, uuid } => {
                let mut is_empty = false;
                if let Some(sender) = subscribed_outputs.get_mut(&output) {
                    sender.remove(&uuid);
                    is_empty = sender.is_empty();
                }
                if is_empty {
                    subscribed_outputs.remove(&output);
                    if let Some(requester) = &requester {
                        unsubscribe(output, &id_tracker, &responder, requester).await;
                    }
                }
            }
            Message::Update { cycler, outputs } => {
                for output in outputs {
                    if let Some(senders) = subscribed_outputs.get(&CyclerOutput {
                        cycler,
                        output: output.output,
                    }) {
                        for sender in senders.values() {
                            sender
                                .send(SubscriberMessage::Update {
                                    value: output.data.clone(),
                                })
                                .await
                                .unwrap()
                        }
                    }
                }
            }
            Message::UpdateOutputHierarchy {
                hierarchy: new_hierarchy,
            } => {
                hierarchy = Some(new_hierarchy);
            }
            Message::GetOutputHierarchy { response_sender } => {
                response_sender.send(hierarchy.clone()).unwrap();
            }
        }
    }
    info!("Finished manager");
}

async fn query_output_hierarchy(
    manager: mpsc::Sender<Message>,
    id_tracker: &mpsc::Sender<id_tracker::Message>,
    responder: &mpsc::Sender<responder::Message>,
    requester: &mpsc::Sender<requester::Message>,
) {
    let message_id = get_message_id(id_tracker).await;
    let (response_sender, response_receiver) = oneshot::channel();
    responder
        .send(responder::Message::Await {
            id: message_id,
            response_sender,
        })
        .await
        .unwrap();
    requester
        .send(requester::Message::GetOutputHierarchy { id: message_id })
        .await
        .unwrap();
    spawn(async move {
        let response = response_receiver.await.unwrap();
        match response {
            Ok(value) => {
                let hierarchy = serde_json::from_value(value);
                match hierarchy {
                    Ok(hierarchy) => {
                        manager
                            .send(Message::UpdateOutputHierarchy { hierarchy })
                            .await
                            .unwrap();
                    }
                    Err(error) => error!("Failed to deserialize OutputHierarchy: {}", error),
                }
            }
            Err(error) => error!("Failed to get output hierarchy: {}", error),
        }
    });
}

async fn add_subscription(
    subscribed_outputs: &mut HashMap<CyclerOutput, HashMap<Uuid, mpsc::Sender<SubscriberMessage>>>,
    uuid: Uuid,
    output: CyclerOutput,
    output_sender: mpsc::Sender<SubscriberMessage>,
    id_tracker: &mpsc::Sender<id_tracker::Message>,
    responder: &mpsc::Sender<responder::Message>,
    requester: &Option<mpsc::Sender<requester::Message>>,
) {
    match subscribed_outputs.entry(output.clone()) {
        Entry::Occupied(mut entry) => {
            entry.get_mut().insert(uuid, output_sender);
        }
        Entry::Vacant(entry) => {
            if let Some(requester) = requester {
                subscribe(
                    output,
                    vec![output_sender.clone()],
                    id_tracker,
                    responder,
                    requester,
                )
                .await;
            };
            entry.insert(HashMap::new()).insert(uuid, output_sender);
        }
    };
}

async fn subscribe(
    output: CyclerOutput,
    subscribers: Vec<mpsc::Sender<SubscriberMessage>>,
    id_tracker: &mpsc::Sender<id_tracker::Message>,
    responder: &mpsc::Sender<responder::Message>,
    requester: &mpsc::Sender<requester::Message>,
) {
    let message_id = get_message_id(id_tracker).await;
    let (response_sender, response_receiver) = oneshot::channel();
    responder
        .send(responder::Message::Await {
            id: message_id,
            response_sender,
        })
        .await
        .unwrap();
    let request = requester::Message::SubscribeOutput {
        id: message_id,
        output,
    };
    requester.send(request).await.unwrap();
    spawn(async move {
        let response = response_receiver.await.unwrap();
        let message = match response {
            Ok(_) => SubscriberMessage::SubscriptionSuccess,
            Err(error) => SubscriberMessage::SubscriptionFailure { info: error },
        };
        for sender in subscribers {
            sender.send(message.clone()).await.unwrap();
        }
    });
}

async fn unsubscribe(
    output: CyclerOutput,
    id_tracker: &mpsc::Sender<id_tracker::Message>,
    responder: &mpsc::Sender<responder::Message>,
    requester: &mpsc::Sender<requester::Message>,
) {
    let message_id = get_message_id(id_tracker).await;
    let (response_sender, response_receiver) = oneshot::channel();
    responder
        .send(responder::Message::Await {
            id: message_id,
            response_sender,
        })
        .await
        .unwrap();
    let request = requester::Message::UnsubscribeOutput {
        id: message_id,
        output,
    };
    requester.send(request).await.unwrap();
    spawn(async move {
        let response = response_receiver.await.unwrap();
        if let Err(error) = response {
            error!("Failed to unsubscribe: {}", error)
        };
    });
}