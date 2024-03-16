use std::fmt::{Display, format};

use iced::{executor, Alignment, Renderer};
use iced::widget::{Column, Row, column, row, Scrollable, scrollable, container, Button, responsive, text};
use iced::{Application, Command, Element, Settings, Theme};

use itertools::Itertools;
use osrs_gph::backend;
use prettytable::Cell;
fn main() {
    println!("{:?}", RustUI::run(Settings::default()));
}

// Godsend https://github.com/tarkah/iced_table/blob/master/example/src/main.rs
// there must be a better way of doing this...
// #[derive(Default)]
struct RustUI{
    // results: ResultsTable<'a, M, T> // Is there any data there?
    results_header: scrollable::Id,
    results_body: scrollable::Id,
    results_titles: Vec<GUIColumn>,
    results_rows: Vec<GUIRow<String>>
}

#[derive(Debug)]
#[derive(Clone)]
enum GUIMessage {
    ProduceResults,
    RefreshResults(prettytable::Table),
    SyncHeader(scrollable::AbsoluteOffset),
}

// TODO: Create wrappers around prettytable::Row
//       to correspond to Column and Row required by iced_table
#[derive(Clone)]
struct GUIColumn {
    kind: ColumnKind,
    align: Alignment,
    width: f32,
    resize_offset: Option<f32>,
} // Hiding columns requires a wrapper around `type Row = Vec<GUICell<String>>`


struct GUIRow<S: Display> {
    data: Vec<GUICell<S>>
}
impl<S: Into<String>, I: IntoIterator<Item = S>> From<I> for GUIRow<String> {
    fn from(value: I) -> Self {
        Self { 
            data: value.into_iter()
            .map(Into::into)
            .map(GUICell::from)
            .collect_vec()
        }
    }
}

// impl From<prettytable::Row> for GUIRow<String> {
//     fn from(value: prettytable::Row) -> Self {
//         value.into_iter()
//     }
// }

#[derive(Clone)]
enum ColumnKind {
    Index(String) // For displaying text
}

impl GUIColumn {
    fn new(kind: ColumnKind, alignment: Option<Alignment>) -> Self {
        let width = match kind {
            ColumnKind::Index(_) => 60.0,
        };

        let align = if let Some(a) = alignment { a } else { Alignment::End };
        Self {
            kind,
            align,
            width,
            resize_offset: None,
        }
    }
}


impl<S: ToString> From<S> for GUIColumn {
    fn from(value: S) -> Self {
        GUIColumn::new(ColumnKind::Index(value.to_string()), None)
    }
}

#[derive(Clone)]
struct GUICell<T: Display> {
    content: T,
    align: Alignment // TODO: Find a way to retrieve alignment 
}

impl<'a> iced_table::table::Column<'a, GUIMessage, Theme, Renderer> for GUIColumn  {
    type Row = GUIRow<String>;
    
    fn header(&'a self, _col_index: usize) -> Element<'a, GUIMessage> {
        #[allow(clippy::infallible_destructuring_match)] // Might end up impl more types
        let content = match &self.kind {
            ColumnKind::Index(header) => header
        };
        let header_height = 24;
        container(text(content)).height(header_height).center_y().into()
    }

    fn footer(
                &'a self,
                _col_index: usize,
                _rows: &'a [Self::Row],
            ) -> Option<Element<'a, GUIMessage, Theme, Renderer>> {
        unimplemented!()
    }
    // When a cell is selected?
    fn cell(
                &'a self,
                col_index: usize,
                row_index: usize,
                row: &'a Self::Row,
            ) -> Element<'a, GUIMessage, Theme, Renderer> {
        todo!()
    }

    fn width(&self) -> f32 {
        self.width
    }

    fn resize_offset(&self) -> Option<f32> {
        self.resize_offset
    }

}

// impl From<prettytable::Cell> for GUICell<String> {
//     fn from(value: prettytable::Cell) -> Self {
//         let content = value.get_content();
//         let align = Alignment::End;
//         // let align = Self::from_pretty_align(value);
//         Self { content, align }
//     }
// }

impl From<String> for GUICell<String> {
    fn from(value: String) -> Self {
        let content = value;
        let align = Alignment::End;
        // let align = Self::from_pretty_align(value);
        Self { content, align }
    }
}

impl<T: Display> Display for GUICell<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.content))
    }
}


impl Application for RustUI{
    type Message = GUIMessage;
    type Flags = (); // No flags currently 2024-03-14
    type Executor = executor::Default;
    type Theme = Theme;

    fn new(_flags: Self::Flags) -> (Self, Command<GUIMessage>) {
        (
            RustUI { 
                results_titles: vec![
                    // GUIColumn::new(ColumnKind::Index("Test".to_string()), None),
                    // GUIColumn::new(ColumnKind::Index("Test2".to_string()), None)
                ],
                results_rows: Vec::new(),
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
                return self.update(GUIMessage::RefreshResults(results_table))
            }
            GUIMessage::RefreshResults(table) => {
                // Should be the title row
                if let Some(names) = table.get_row(0) {
                    self.results_titles = names.into_iter()
                    .map(Cell::get_content)
                    .map(GUIColumn::from)
                    .collect_vec();

                    // Assuming rest of table is well formatted
                    // Need to skip the first row
                    let table_content =
                    table.row_iter().skip(1)
                        .map(|r| 
                            r.into_iter().map(std::string::ToString::to_string).collect_vec()
                        )
                        .map(GUIRow::from).collect_vec();
                    self.results_rows = table_content;
                };
            }
            GUIMessage::SyncHeader(offset) => {
                return Command::batch(vec![
                    scrollable::scroll_to(self.results_header.clone(), offset),
                    scrollable::scroll_to(self.results_body.clone(), offset),
                ])
            }
        }

        Command::none()
    }

    fn view(&self) -> Element<'_, Self::Message, Self::Theme, iced::Renderer> {
        // let content =
        //     column![results_table].spacing(20);
        container(
            responsive(|_| {
                let results_table: iced_table::Table<'_, GUIColumn, GUIRow<String>, GUIMessage, Theme> =
                    iced_table::table(
                    self.results_header.clone(), 
                    self.results_body.clone(), 
                    &self.results_titles, 
                    &self.results_rows, 
                    GUIMessage::SyncHeader
                );
                results_table.into()
    })
        ).into()
    }
}