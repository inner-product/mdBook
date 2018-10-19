use super::{Preprocessor, PreprocessorContext};
use book::{Book, BookItem, Chapter};
use errors::{Error, Result};
use pulldown_cmark::{Event, Parser, Tag};
use pulldown_cmark_to_cmark::fmt::cmark;
use regex::Regex;

enum State {
    OutsideScala,
    ScalaFirstLine,
    InsideWrapped,
    InsideUnwrapped,
}

/// A preprocessor to remove `object wrapper` in Scala code blocks
pub struct ScalaWrapperPreprocessor;

impl ScalaWrapperPreprocessor {
    /// Create a new instance of the Scala wrapper preprocessor
    pub fn new() -> ScalaWrapperPreprocessor {
        ScalaWrapperPreprocessor
    }

    fn remove_wrappers(&self, chapter: &mut Chapter) -> Result<String> {
        let wrapper_start = Regex::new(r"object wrapper.*\{").unwrap();
        let wrapper_end = Regex::new(r"^\}").unwrap();
        let mut buf = String::with_capacity(chapter.content.len());
        let mut state: State = State::OutsideScala;

        let events = Parser::new(&chapter.content).filter_map(|event| match event {
            Event::Start(Tag::CodeBlock(lang)) => {
                if lang.as_ref() == "scala" {
                    state = State::ScalaFirstLine;
                }
                Some(Event::Start(Tag::CodeBlock(lang)))
            }

            Event::Text(content) => match state {
                State::OutsideScala => Some(Event::Text(content)),

                State::ScalaFirstLine => {
                    if wrapper_start.is_match(&content) {
                        state = State::InsideWrapped;
                        None
                    } else {
                        state = State::InsideUnwrapped;
                        Some(Event::Text(content))
                    }
                }

                State::InsideWrapped => {
                    if wrapper_end.is_match(&content) {
                        None
                    } else {
                        Some(Event::Text(content))
                    }
                }

                State::InsideUnwrapped => Some(Event::Text(content)),
            },

            Event::End(Tag::CodeBlock(_)) => {
                state = State::OutsideScala;
                Some(event)
            }
            other => Some(other),
        });

        cmark(events, &mut buf, None).map(|_| buf).map_err(|err| {
            Error::from(format!(
                "Markdown serialization failed within {}: {}",
                self.name(),
                err
            ))
        })
    }
}

impl Preprocessor for ScalaWrapperPreprocessor {
    fn name(&self) -> &str {
        "scala-wrapper-preprocessor"
    }

    fn run(&self, _ctx: &PreprocessorContext, book: &mut Book) -> Result<()> {
        eprintln!("Running '{}' preprocessor", self.name());
        let mut result: Result<()> = Ok(());
        let mut error = false;

        book.for_each_mut(|item: &mut BookItem| {
            if error {
                return;
            } else {
                if let BookItem::Chapter(ref mut chapter) = *item {
                    eprintln!("{}: processing chapter '{}'", self.name(), chapter.name);
                    result = match self.remove_wrappers(chapter) {
                        Ok(content) => {
                            chapter.content = content;
                            Ok(())
                        }

                        Err(err) => {
                            error = true;
                            Err(err)
                        }
                    }
                }
            }
        });

        result
    }
}
