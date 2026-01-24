mod send_request;
mod sum;

use std::sync::Arc;
use iced::{Border, Color, Fill, FillPortion, Shadow, Task};
use iced::alignment::Horizontal;
use iced::widget::{text_editor, text, radio, row, column, container, button, scrollable};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum Pages {
    Question {},
    FirstAnswer {},
    Eval {},
    FinalAnswer {}
}

impl Pages {
    fn as_str(&self) -> &str {
        match self {
            Self::Question {} => "question",
            Self::FirstAnswer {} => "first answer",
            Self::Eval {} => "eval",
            Self::FinalAnswer {} => "final answer",
        }
    }
}

#[derive(Debug, Clone)]
enum Message {
    ChangePage(Pages),
    EditQuestion(text_editor::Action),
    AskQuestion,
    FirstSum(Vec<Option<String>>),
    AskEval(Vec<Option<String>>),
    SecondSum(Vec<Option<String>>),
    AskBest(Vec<Option<String>>),
    GetFinal(Option<String>)
}

struct App {
    question: text_editor::Content,
    page: Pages,
    ans_question: [String; 3],
    sum_questions: [String; 3],
    evals: [String; 3],
    sum_evals: [String; 3],
    best_answer: String,
    best_num: usize
}

/// Main event loop
impl App {
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ChangePage(new) => { 
                self.page = new; 
                Task::none()
            }
            Message::EditQuestion(action) => {
                self.question.perform(action);
                Task::none()
            }
            Message::AskQuestion => {
                let prompt = "You are one of several assistants, solving one question. Write plain text, no emoji or markdown.".to_string();
                let text = self.question.text();
                Task::perform(
                    send_request::send_requests(prompt, text),
                    |result| Message::FirstSum(result)
                )
            }
            Message::FirstSum(answers) => {
                self.page = Pages::FirstAnswer {};
                let mut next = true;
                for (num, res) in answers.into_iter().enumerate() {
                    if let Some(text) = res {
                        self.ans_question[num] = text;
                    } else {
                        self.ans_question[num] = "I'm sorry I died :(".to_string();
                        next = false
                    }
                }
                
                if next {
                    let texts = self.ans_question.clone();
                    Task::perform(
                        sum::summary(texts),
                        |result| Message::AskEval(result)
                    )
                } else {
                    Task::none()
                }
            }
            Message::AskEval(sums) => {
                let mut next = true;
                for (num, i) in sums.into_iter().enumerate() {
                    if let Some(sum) = i {
                        self.sum_questions[num] = sum;
                    }
                    else {
                        next = false;
                        self.sum_questions[num] = "No sum".to_string();
                    }
                };
                
                if next {
                    let prompt = "You have to analyze this 3 texts and say which is the best and why. \
                        End with \"The best is <number>.\". Write plain text, no emoji or markdown.".to_string();
                    let text = format!("Text number 1: {} \nText number 2: {} \nText number 3: {}", 
                        self.sum_questions[0], self.sum_questions[1], self.sum_questions[2]);
                    Task::perform(
                        send_request::send_requests(prompt, text),
                        |result| Message::SecondSum(result)
                    )
                } else {
                    Task::none()
                }
            }
            Message::SecondSum(answers) => {
                self.page = Pages::FirstAnswer {};
                let mut next = true;
                for (num, res) in answers.into_iter().enumerate() {
                    if let Some(text) = res {
                        self.evals[num] = text;
                    } else {
                        self.evals[num] = "I'm sorry I died :(".to_string();
                        next = false
                    }
                }

                if next {
                    let texts = self.ans_question.clone();
                    Task::perform(
                        sum::summary(texts),
                        |result| Message::AskBest(result)
                    )
                } else {
                    Task::none()
                }
            }
            Message::AskBest(sums) => {
                self.page = Pages::Eval {};
                for (num, i) in sums.into_iter().enumerate() {
                    if let Some(sum) = i {
                        self.sum_evals[num] = sum;
                    }
                    else {
                        self.sum_evals[num] = "No sum".to_string();
                    }
                };
                
                let mut scores = vec![];
                for i in self.evals.iter() {
                    let mut last = *i.as_bytes().last().unwrap();
                    if last < '0' as u8 && last > '3' as u8 {
                        last = *i.as_bytes().get(i.len() - 2).unwrap();
                    }
                    println!("last {}", last);
                    if last >= '1' as u8 && last <= '3' as u8 {
                        scores.push(last - '0' as u8)
                    }
                }
                println!("{:?}", scores);
                self.best_num = match scores.len() {
                    0 => 0,
                    1 | 2 => scores[0] as usize - 1,
                    _ => {
                        if scores[0] == scores[1] || scores[0] == scores[2] {
                            scores[0] as usize - 1
                        }
                        else {
                            scores[1] as usize - 1
                        }
                    }
                };
                
                let prompt = "You were chosen as the best answerer, provide a mid-sized answer to a question. You can use provided reviews as helpers. ".to_string();
                let text = format!("The question itself is: {} \n \
                Answer number 1: {}\n Answer number 2: {}\n Answer number 3: {}\n\
                Review 1: {} \n Review 2: {} \n Review 3: {} \n",
                    self.question.text(), self.sum_questions[0], self.sum_questions[1], self.sum_questions[2],
                    self.sum_evals[0], self.sum_evals[1], self.sum_evals[2]
                );
                match self.best_num {
                    0 =>
                        Task::perform(
                            send_request::ask_gemini(Arc::new(prompt), Arc::new(text)),
                            |ans| Message::GetFinal(ans)
                        ),
                    1 =>
                        Task::perform(
                            send_request::ask_grok(Arc::new(prompt), Arc::new(text)),
                            |ans| Message::GetFinal(ans)
                        ),
                    _ =>
                        Task::perform(
                            send_request::ask_mistral(Arc::new(prompt), Arc::new(text)),
                            |ans| Message::GetFinal(ans)
                        )
                }
                
            }
            Message::GetFinal(ans) => {
                self.page = Pages::FinalAnswer {};
                if let Some(answer) = ans {
                    self.best_answer = answer
                }
                else {
                    self.best_answer = self.ans_question[self.best_num].clone()
                }
                Task::none()
            }
        }
    }
    
    fn view(&self) -> iced::Element<Message> {
        let pager = row![
            radio(Pages::Question {}.as_str(), Pages::Question {}, Some(self.page), Message::ChangePage), 
            radio(Pages::FirstAnswer {}.as_str(), Pages::FirstAnswer {}, Some(self.page), Message::ChangePage), 
            radio(Pages::Eval {}.as_str(), Pages::Eval {}, Some(self.page), Message::ChangePage), 
            radio(Pages::FinalAnswer {}.as_str(), Pages::FinalAnswer {}, Some(self.page), Message::ChangePage)
        ].width(Fill);
        
        let main_info = match self.page {
            Pages::Question {}  => self.view_question(),
            Pages::FirstAnswer {} => self.view_text(&self.ans_question, &self.sum_questions),
            Pages::Eval {} => self.view_text(&self.evals, &self.sum_evals),
            Pages::FinalAnswer {} => self.view_final()
        };
        column![pager, main_info].into()
    }
}

/// Different view cases
impl App {
    fn view_question(&self) -> container::Container<Message> {
        container(
            column![
            container(text("Ask Your Question").size(20))
                .padding(12)
                .width(Fill)
                .style(|_theme| container::Style {
                    background: Some(Color::from_rgb(0.2, 0.6, 0.86).into()),
                    text_color: Some(Color::WHITE),
                    border: Border::default(),
                    shadow: Shadow::default(),
                }),
            container(
                text_editor(&self.question)
                    .placeholder("Write your question here")
                    .on_action(Message::EditQuestion)
            )
            .padding(10)
            .style(|_theme| container::Style {
                background: Some(Color::WHITE.into()),
                border: Border::default()
                    .width(2.0)
                    .color(Color::from_rgb(0.2, 0.6, 0.86))
                    .rounded(4.0),
                shadow: Shadow::default(),
                text_color: Some(Color::BLACK),
            }),
            container(
                button(
                    text("Send Question")
                        .size(16)
                        .align_x(Horizontal::Center)
                )
                .on_press(Message::AskQuestion)
                .padding(12)
                .style(|_theme, _status| button::Style {
                    background: Some(Color::from_rgb(0.2, 0.6, 0.86).into()),
                    text_color: Color::WHITE,
                    border: Border::default().rounded(6.0),
                    shadow: Shadow::default(),
                })
            )
            .padding([10, 0])
            .width(Fill)
            .align_x(Horizontal::Center),
        ]
                .spacing(15)
        )
            .padding(20)
            .style(|_theme| container::Style {
                background: Some(Color::from_rgb(0.92, 0.96, 0.99).into()),
                border: Border::default()
                    .width(2.0)
                    .color(Color::from_rgb(0.2, 0.6, 0.86)),
                shadow: Shadow::default(),
                text_color: Some(Color::BLACK),
            })
            .width(Fill)
    }
    
    fn view_text(&self, full_text: &[String; 3], sum: &[String; 3]) -> container::Container<Message> {
        container(
            row![
                container(
                    column![
                        container(text("Gemini says:").size(18))
                            .padding(10)
                            .style(|_theme| container::Style {
                                background: Some(Color::from_rgb(0.26, 0.52, 0.96).into()),
                                text_color: Some(Color::WHITE),
                                ..Default::default()
                            }),
                        scrollable(
                            text(full_text[0].clone())
                                .size(14)
                        )
                        .height(Fill),
                        container(text("Shorter version:").size(16))
                            .padding(8)
                            .style(|_theme| container::Style {
                                background: Some(Color::from_rgb(0.3, 0.56, 0.98).into()),
                                text_color: Some(Color::WHITE),
                                ..Default::default()
                            }),
                        scrollable(
                            text(sum[0].clone())
                                .size(14)
                        )
                        .height(Fill),
                    ]
                    .spacing(10)
                )
                .padding(15)
                .style(|_theme| container::Style {
                    background: Some(Color::from_rgb(0.9, 0.94, 0.99).into()),
                    border: Border::default()
                        .width(2.0)
                        .color(Color::from_rgb(0.26, 0.52, 0.96)),
                    shadow: Shadow::default(),
                    text_color: Some(Color::BLACK),
                })
                .width(FillPortion(1)),
        
                container(
                    column![
                        container(text("Grok says:").size(18))
                            .padding(10)
                            .style(|_theme| container::Style {
                                background: Some(Color::from_rgb(0.3, 0.69, 0.31).into()),
                                text_color: Some(Color::WHITE),
                                ..Default::default()
                            }),
                        scrollable(
                            text(full_text[1].clone())
                                .size(14)
                        )
                        .height(Fill),
                        container(text("Shorter version:").size(16))
                            .padding(8)
                            .style(|_theme| container::Style {
                                background: Some(Color::from_rgb(0.4, 0.74, 0.42).into()),
                                text_color: Some(Color::WHITE),
                                ..Default::default()
                            }),
                        scrollable(
                            text(sum[1].clone())
                                .size(14)
                        )
                        .height(Fill),
                    ]
                    .spacing(10)
                )
                .padding(15)
                .style(|_theme| container::Style {
                    background: Some(Color::from_rgb(0.9, 0.97, 0.9).into()),
                    border: Border::default()
                        .width(2.0)
                        .color(Color::from_rgb(0.3, 0.69, 0.31)),
                    shadow: Shadow::default(),
                    text_color: Some(Color::BLACK),
                })
                .width(FillPortion(1)),
        
                container(
                    column![
                        container(text("Mistral says:").size(18))
                            .padding(10)
                            .style(|_theme| container::Style {
                                background: Some(Color::from_rgb(0.61, 0.15, 0.69).into()),
                                text_color: Some(Color::WHITE),
                                border: Border::default(),
                                shadow: Shadow::default(),
                            }),
                        scrollable(
                            text(full_text[2].clone())
                                .size(14)
                        )
                        .height(Fill),
                        container(text("Shorter version:").size(16))
                            .padding(8)
                            .style(|_theme| container::Style {
                                background: Some(Color::from_rgb(0.67, 0.28, 0.75).into()),
                                text_color: Some(Color::WHITE),
                                ..Default::default()
                            }),
                        scrollable(
                            text(sum[2].clone())
                                .size(14)
                        )
                        .height(Fill),
                    ]
                    .spacing(10)
                )
                .padding(15)
                .style(|_theme| container::Style {
                    background: Some(Color::from_rgb(0.96, 0.9, 0.98).into()),
                    border: Border::default()
                        .width(2.0)
                        .color(Color::from_rgb(0.61, 0.15, 0.69)),
                    shadow: Shadow::default(),
                    text_color: Some(Color::BLACK),
                })
                .width(FillPortion(1)),
            ]
                .spacing(15)
        )
            .padding(20)
            .style(|_| container::Style {
                background: Some(Color::from_rgb(0.95, 0.95, 0.95).into()),
                text_color: Some(Color::BLACK),
                ..Default::default()
            })
            .width(Fill)
            .height(Fill)
    }
    
    fn view_final(&self) -> container::Container<Message> {
        let who = match self.best_num {
            0 => "Gemini",
            1 => "Grok",
            2 => "Mistral",
            _ => ""
        };
        container(
            column![
        container(
            text(format!("The Final Answer by {}", who))
                .size(22)
                .align_x(Horizontal::Center)
        )
        .padding(15)
        .width(Fill)
        .style(|_theme| container::Style {
            background: Some(Color::from_rgb(0.85, 0.37, 0.0).into()),
            text_color: Some(Color::WHITE),
            border: Border::default(),
            shadow: Shadow::default(),
        }),
        container(
            scrollable(
                text(&self.best_answer)
                    .size(15)
            )
            .height(Fill)
        )
        .padding(20)
        .width(Fill)
        .height(Fill)
        .style(|_theme| container::Style {
            background: Some(Color::WHITE.into()),
            border: Border::default()
                .width(1.0)
                .color(Color::from_rgb(0.9, 0.9, 0.9)),
            shadow: Shadow::default(),
            text_color: Some(Color::BLACK),
        }),
    ]
                .spacing(0)
        )
            .style(|_theme| container::Style {
                background: Some(Color::from_rgb(0.99, 0.95, 0.9).into()),
                border: Border::default()
                    .width(3.0)
                    .color(Color::from_rgb(0.85, 0.37, 0.0)),
                shadow: Shadow::default(),
                text_color: Some(Color::BLACK),
            })
            .padding(20)
            .width(Fill)
            .height(Fill)
    }
}

/// Helper functions for styling
impl App {
    
}

impl Default for App {
    fn default() -> Self {
        Self {
            question: text_editor::Content::new(),
            page: Pages::Question{},
            ans_question: [String::default(), String::default(), String::default()],
            sum_questions: [String::default(), String::default(), String::default()],
            evals: [String::default(), String::default(), String::default()],
            sum_evals: [String::default(), String::default(), String::default()],
            best_answer: "".to_string(),
            best_num: 10
        }
    }
}

fn main() -> iced::Result {
    iced::application("Brain storm", App::update, App::view).run()
}
