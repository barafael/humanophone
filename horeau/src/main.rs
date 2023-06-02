#![doc = include_str!("../README.md")]

use anyhow::Error;
use klib::core::chord::Chord;
use morivar::ConsumerMessage;
//use morivar::{ConsumerMessage, PublisherMessage};
use serde::Deserialize;

use yew::{html, Component, Context, Html};
use yew_websocket::macros::Json;
use yew_websocket::websocket::{WebSocketService, WebSocketStatus, WebSocketTask};
pub enum WsAction {
    Connect,
    Identify(String),
    Disconnect,
    Lost,
}

pub enum Msg {
    WsAction(WsAction),
    WsReady(Result<WsResponse, Error>),
}

impl From<WsAction> for Msg {
    fn from(action: WsAction) -> Self {
        Msg::WsAction(action)
    }
}

/// This type is an expected response from a websocket connection.
#[derive(Deserialize, Debug)]
pub struct WsResponse {
    value: Chord,
}

pub struct Model {
    pub data: Option<Chord>,
    pub ws: Option<WebSocketTask>,
}

impl Model {
    fn view_data(&self) -> Html {
        if let Ok(value) = serde_json::to_string_pretty(&self.data) {
            html! {
                <p>{ value }</p>
            }
        } else {
            html! {
                <p>{ "Data hasn't fetched yet." }</p>
            }
        }
    }
}

impl Component for Model {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            data: None,
            ws: None,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::WsAction(action) => match action {
                WsAction::Connect => {
                    let callback = ctx.link().callback(|Json(data)| Msg::WsReady(data));
                    let notification = ctx.link().batch_callback(|status| match status {
                        WebSocketStatus::Opened => None,
                        WebSocketStatus::Closed | WebSocketStatus::Error => {
                            Some(WsAction::Lost.into())
                        }
                    });
                    let task = WebSocketService::connect(
                        "wss://humanoph.one:8000",
                        callback,
                        notification,
                    )
                    .unwrap();
                    self.ws = Some(task);
                    true
                }
                WsAction::Identify(id) => {
                    let message = ConsumerMessage::IAmConsumer { id };
                    self.ws
                        .as_mut()
                        .unwrap()
                        .send(serde_json::to_string(&message).unwrap());
                    false
                }
                WsAction::Disconnect => {
                    self.ws.take();
                    true
                }
                WsAction::Lost => {
                    self.ws = None;
                    true
                }
            },
            Msg::WsReady(response) => {
                self.data = response.map(|data| data.value).ok();
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div>
                <nav class="menu">
                    { self.view_data() }
                    <button disabled={self.ws.is_some()}
                            onclick={ctx.link().callback(|_| WsAction::Connect)}>
                        { "Connect To Humanophone" }
                    </button>
                    <button disabled={self.ws.is_none()}
                            onclick={ctx.link().callback(|_| WsAction::Identify("Horeau".to_string()))}>
                        { "Identify as Consumer" }
                    </button>
                    <button disabled={self.ws.is_none()}
                            onclick={ctx.link().callback(|_| WsAction::Disconnect)}>
                        { "Close WebSocket connection" }
                    </button>
                </nav>
            </div>
        }
    }
}

fn main() {
    yew::Renderer::<Model>::new().render();
}
