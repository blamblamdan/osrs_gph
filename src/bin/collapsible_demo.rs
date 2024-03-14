use iced::{Alignment, widget::Column, Element, Sandbox, Settings};

pub fn main() {
    let _ = Counters::run(Settings::default());
}

mod collapsible {
    use iced::{Alignment, widget::Button, widget::Column, Element, Length, widget::Row, widget::Text};

    #[derive(Debug, Clone, Copy)]
    pub enum Message<B> {
        Toggle,
        Body(B),
    }

    pub struct Collapsible {
        title: String,
        is_expanded: bool,
    }

    impl Collapsible {
        pub fn new(title: String, is_expanded: bool) -> Self {
            Self {
                title,
                is_expanded,
            }
        }

        pub fn toggle(&mut self) {
            self.is_expanded = !self.is_expanded;
        }

        pub fn view<'s, MessageBody: 'static>(
            &'s self,
            view_body: impl FnOnce() -> Element<'s, MessageBody>,
        ) -> Element<'s, Message<MessageBody>>
        where
            MessageBody: Clone,
        {
            let header = Button::new(
                Row::new()
                    .spacing(20)
                    .align_items(Alignment::Center)
                    .push(Text::new(&self.title).width(Length::Fill))
                    .push(Text::new(if self.is_expanded {
                        "↓"
                    } else {
                        "←"
                    })),
            )
            .on_press(Message::Toggle);

            if self.is_expanded {
                Column::with_children(vec![
                    header.into(),
                    view_body().map(|msg| Message::Body(msg)),
                ])
                .into()
            } else {
                header.into()
            }
        }
    }
}

mod counter {
    use iced::{Alignment, widget::Button, Element, widget::Row, widget::Text};

    #[derive(Default)]
    pub struct Counter {
        value: i32,
    }

    #[derive(Debug, Clone, Copy)]
    pub enum Message {
        IncrementPressed,
        DecrementPressed,
    }

    impl Counter {
        const BUTTON_SIZE: u16 = 50;
        pub fn update(&mut self, message: Message) {
            match message {
                Message::IncrementPressed => {
                    self.value += 1;
                }
                Message::DecrementPressed => {
                    self.value -= 1;
                }
            }
        }

        pub fn view(&self) -> Element<Message> {
            Row::new()
                .padding(20)
                .align_items(Alignment::Center)
                .push(
                    Button::new(Text::new("+"))
                        .on_press(Message::IncrementPressed),
                )
                .push(Text::new(self.value.to_string()).size(Self::BUTTON_SIZE))
                .push(
                    Button::new(Text::new("-"))
                        .on_press(Message::DecrementPressed),
                )
                .into()
        }
    }
}

use collapsible::Collapsible;
use counter::Counter;

struct Counters {
    counters: Vec<(Collapsible, Counter)>,
}

#[derive(Debug)]
enum GUIMessage {
    CounterToggle(usize, collapsible::Message<counter::Message>)
}
impl Sandbox for Counters {
    type Message = GUIMessage;

    fn new() -> Self {
        Self {
            counters: ["situps", "pushups", "jumps", "cats", "dogs"]
                .iter()
                .map(|name| {
                    (
                        Collapsible::new((*name).to_string(), false),
                        Counter::default(),
                    )
                })
                .collect(),
        }
    }

    fn title(&self) -> String {
        String::from("Counter - Iced")
    }

    fn update(&mut self, message: Self::Message) {
        match message {
            GUIMessage::CounterToggle(idx, msg) => { // Might need to make a tuple
                match msg {
                    collapsible::Message::Toggle => {
                        self.counters[idx].0.toggle();
                    }
                    collapsible::Message::Body(msg) => {
                        self.counters[idx].1.update(msg);
                    }
                }
            }
        }
    }

    fn view(&self) -> Element<Self::Message> {
        Column::with_children(
            self.counters
                .iter()
                .enumerate()
                .map(|(i, (collapsible, counter))| {
                    collapsible
                        .view(move || counter.view())
                        .map(move |msg| Self::Message::CounterToggle(i, msg))
                })
                .collect::<Vec<_>>(),
        )
        .align_items(Alignment::Start)
        .into()
    }
}
