mod request;

use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, AtomicI16, Ordering};
use std::time::Duration;
use iced::widget::{text, button, column, row, scrollable, text_editor, vertical_space};
use iced::{Task, Subscription, Fill, FillPortion, Center};
use iced::time::every;

#[derive(Debug, Clone)]
enum Message {
    GetData,
    Done,
    None,
    Writing(text_editor::Action),
    Restart
}

struct App {
    status: &'static str,
    content: text_editor::Content,
    sites: Arc<[&'static str; 3]>,
    container: Arc<[Mutex<String>; 3]>,
    text: Arc<String>,
    counter: Arc<AtomicI16>,
    ready: Arc<[AtomicBool; 3]>
}

impl App {
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::GetData => {
                self.status = "Processing";
                self.text = Arc::new(self.content.text());
                self.ready.iter().for_each(|x| x.store(false, Ordering::Release));
                self.counter.store(0, Ordering::Release);
                self.container.iter().for_each(|mutex| {
                    let mut x = mutex.lock().expect("No lock");
                    *x = String::default();
                });

                Task::perform(
                    request::request_sum(self.sites.clone(), self.container.clone(), self.text.clone(), self.counter.clone(), self.ready.clone()),
                    |_| Message::Done
                )
            }
            Message::None => {
                Task::none()
            }
            Message::Done => {
                self.status = "Done";
                Task::none()
            }
            Message::Writing(action) => {
                self.content.perform(action);
                Task::none()
            }
            Message::Restart => {
                self.status = "Waiting";
                self.content = text_editor::Content::with_text("Write here");
                Task::none()
            }
        }
    }
    
    fn view (&'_ self) -> iced::Element<'_, Message> {
        let mut column = column![text("Text summary")].align_x(Center).padding(5);
        column = column.push(text(format!("Статус: {}", self.status)));
        if self.status == "Waiting" {
            let send = button("Отправить запрос").on_press(Message::GetData);
            let main_text = text_editor(&self.content).on_action(Message::Writing);
            column = column.push(send).push(main_text);
        }
        else if self.status == "Processing" || self.status == "Done" {
            let timer = text(format!("Ожидание ответа: {} секунд", self.counter.load(Ordering::Relaxed)));
            
            let main_text = scrollable(text(self.text.clone().as_str().to_string()));
            
            column = column.push(timer)
                .push(text("Изначальный текст:").size(20).width(Fill))
                .push(main_text);
            
            let sum_text = self.container
                .iter()
                .map(|sum| return sum.lock().expect("No lock").clone())
                .collect::<Vec<String>>();
            
            let scrolls: Vec<_> = sum_text
                .into_iter()
                .map(|sum| {scrollable(text(sum.as_str().to_string()))})
                .collect();
            
            let mut scroll_row = row![].spacing(8).padding(10);
            for (num, scroll) in scrolls.into_iter().enumerate() {
                let mut summary = column![text(format!("Суммаризация от {}:", self.sites[num])).size(20)];
                summary = summary.push(scroll.width(FillPortion(1)));
                scroll_row = scroll_row.push(vertical_space().height(Fill)).push(summary);
            }
            column = column.push(scroll_row.width(Fill));
            
            if self.status == "Done" {
                let restart = button("Отправить другой текст").on_press(Message::Restart);
                column = column.push(restart);
            }
        }
        column.into()
    }
    
    fn subscription (&self) -> Subscription<Message> {
        if self.status == "Processing" {
            every(Duration::from_millis(250)).map(|_a| Message::None)
        } else {
            Subscription::none()
        }
    }
}

impl Default for App {
    fn default() -> Self {
        Self {
            status: "Waiting",
            content: text_editor::Content::with_text("Hello there"),
            sites: Arc::new(["distilbart", "gemini", "cohere"]),
            container: Arc::new([Mutex::new(String::default()), Mutex::new(String::default()), Mutex::new(String::default())]),
            text: Arc::default(),
            counter: Arc::new(AtomicI16::new(0)),
            ready: Arc::new([const { AtomicBool::new(false) }; 3])
        }
    }
}

#[tokio::main]
async fn main() {
    iced::application("Lab 1", App::update, App::view)
        .subscription(App::subscription)
        .theme(|_|{ iced::theme::Theme::Dracula})
        .run()
        .unwrap();
}
