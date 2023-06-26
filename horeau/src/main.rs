#![doc = include_str!("../README.md")]

use std::time::Duration;

use anyhow::Error;
use klib::core::base::{HasName, Playable, PlaybackHandle};
use morivar::ConsumerMessage;
//use morivar::{ConsumerMessage, PublisherMessage};

use yew::{html, Component, Context, Html};
use yew_websocket::macros::Json;
use yew_websocket::websocket::{WebSocketService, WebSocketStatus, WebSocketTask};
pub enum WsAction {
    Connect,
    Identify(String),
    Disconnect,
    Lost,
}
use tracing_subscriber::{
    fmt::format::{FmtSpan, Pretty},
    prelude::*,
};
use wasm_bindgen::JsValue;

pub enum Msg {
    WsAction(WsAction),
    WsReady(Result<WsResponse, Error>),
}

impl From<WsAction> for Msg {
    fn from(action: WsAction) -> Self {
        Self::WsAction(action)
    }
}

/// This type is an expected response from a websocket connection.
pub type WsResponse = ConsumerMessage;

pub struct Model {
    pub data: Option<ConsumerMessage>,
    pub ws: Option<WebSocketTask>,
    pub handle: Option<PlaybackHandle>,
}

impl Model {
    fn view_data(&self) -> Html {
        if let Some(ConsumerMessage::ChordEvent(chord)) = &self.data {
            html!(
                <p>{ format!("{}", chord.name()) }</p>
            )
        } else if let Ok(value) = serde_json::to_string_pretty(&self.data) {
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
            handle: None,
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
                    let task =
                        WebSocketService::connect("wss://humanoph.one:443", callback, notification)
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
                tracing::info!("{response:?}");
                self.data = response.ok();
                if let Some(ConsumerMessage::ChordEvent(chord)) = &self.data {
                    let handle = chord
                        .play(
                            Duration::ZERO,
                            Duration::from_secs(4),
                            Duration::from_millis(100),
                        )
                        .unwrap();
                    self.handle = Some(handle);
                }
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
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .without_time()
        .with_writer(tracing_web::MakeConsoleWriter)
        .with_span_events(FmtSpan::ACTIVE);
    let perf_layer = tracing_web::performance_layer().with_details_from_fields(Pretty::default());

    tracing_subscriber::registry()
        .with(fmt_layer)
        .with(perf_layer)
        .init();
    let object = JsValue::from("world");
    tracing::info!("Hello {}", object.as_string().unwrap());

    yew::Renderer::<Model>::new().render();
}
