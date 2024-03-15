use iced::executor;
use iced::widget::{Column, Row, column, row, Scrollable, scrollable, container, Button, responsive};
use iced::{Application, Command, Element, Settings, Theme};

use itertools::Itertools;
use osrs_gph::backend;
fn main() {

}

// Godsend https://github.com/project-gauntlet/iced_table/blob/master/example/src/main.rs
type ResultsTable<'a, M, T> = iced_table::Table<'a, Column<'a, M>, Row<'a, M>, M, T>;
// there must be a better way of doing this...
// #[derive(Default)]
struct RustUI{
    // results: ResultsTable<'a, M, T> // Is there any data there?
    results_header: scrollable::Id,
    results_body: scrollable::Id,
    results: prettytable::Table
}

#[derive(Debug)]
#[derive(Clone)]
enum GUIMessage {
    ProduceResults,
    RefreshResuls(prettytable::Table),
    ScrollTo(scrollable::AbsoluteOffset)
}

// TODO: Create wrappers around prettytable::Row
//       to correspond to Column and Row required by iced_table


impl Application for RustUI{
    type Message = GUIMessage;
    type Flags = (); // No flags currently 2024-03-14
    type Executor = executor::Default;
    type Theme = Theme;

    fn new(flags: Self::Flags) -> (Self, Command<GUIMessage>) {
        (
            RustUI { 
                results: prettytable::Table::new(),
                results_header: scrollable::Id::unique(),
                results_body: scrollable::Id::unique()
            },
            Command::none()
            // font::load(include_bytes!("../fonts/icons.ttf").as_slice())
            //     .map(Message::FontLoaded),
        )
    }

    fn title(&self) -> String {
        String::from("OSRS_GPH - Money Making Calculator")
    }

    fn update(&mut self, message: GUIMessage) -> Command<GUIMessage> {
        match message {
            GUIMessage::ProduceResults => {
                let (_,_, results_table) = backend::main_inner();
                return self.update(GUIMessage::RefreshResuls(results_table))
            }
            GUIMessage::RefreshResuls(table) => {
                self.results = table;
            }
            GUIMessage::ScrollTo(offset) => {
                return Command::batch(vec![
                    scrollable::scroll_to(self.results_header.clone(), offset),
                    scrollable::scroll_to(self.results_body.clone(), offset),
                ])
            }
        }

        Command::none()
    }

    fn view(&self) -> Element<'_, Self::Message, Self::Theme, iced::Renderer> {
        // Convert prettytable::
        let table_content =
            self.results.row_iter().collect_vec();

        // Should be the title row
        let col_names = self.results.get_row(0).unwrap().to_owned().into_iter()
            .map(|x| x.to_string())
            .collect_vec();
        let results_table: iced_table::Table<'_, _, _, GUIMessage, Theme> =
            iced_table::table(
            self.results_header, 
            self.results_body, 
            &col_names, 
            table_content.as_slice(), 
            GUIMessage::ScrollTo
        );

        todo!();
        // let content =
        //     column![results_table].spacing(20);
        container(
            results_table
        ).into()
    }
}