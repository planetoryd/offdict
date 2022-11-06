use iced::widget::{Button, Column, Container, Slider};
use iced::{Color, Element, Length, Renderer, Sandbox, Settings, container};

pub fn main() -> iced::Result {
    Offdict::run(Settings::default())
}

impl Sandbox for Offdict {
    type Message = Message;

    fn new() -> Self {
        Offdict {
            query: "".to_owned(),
        }
    }

    fn title(&self) -> String {
        "Offdict".to_owned()
    }

    fn view(&mut self) -> Element<'_, Self::Message> {
        let scrollable = scrollable(
            container(if self.debug {
                content.explain(Color::BLACK)
            } else {
                content
            })
            .width(Length::Fill)
            .center_x(),
        );
        
        container::Container::new(scrollable)
    }

    fn update(&mut self, message: Self::Message) {
        
    }
}

pub struct Offdict {
    query: String,
}

#[derive(Debug, Clone)]
pub enum Message {
    QueryChange,
}
