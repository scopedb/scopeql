// Copyright 2025 ScopeDB, Inc.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::borrow::Cow;
use std::sync::Arc;
use std::sync::Mutex;

use nu_ansi_term::Style;
use reedline::DefaultHinter;
use reedline::Hinter;
use reedline::History;
use reedline::Prompt;
use reedline::PromptEditMode;
use reedline::PromptHistorySearch;
use reedline::PromptHistorySearchStatus;

#[derive(Debug, Clone)]
pub struct PromptRenderState {
    line: String,
    input_is_empty: bool,
}

impl PromptRenderState {
    pub fn new(line: String) -> Self {
        Self {
            line,
            input_is_empty: true,
        }
    }

    pub fn set_line(&mut self, line: String) {
        self.line = line;
    }

    fn set_input_is_empty(&mut self, input_is_empty: bool) {
        self.input_is_empty = input_is_empty;
    }

    fn input_is_empty(&self) -> bool {
        self.input_is_empty
    }

    fn line(&self) -> &str {
        &self.line
    }
}

#[derive(Debug)]
pub struct CommandLinePrompt {
    state: Arc<Mutex<PromptRenderState>>,
}

impl CommandLinePrompt {
    pub fn new(state: Arc<Mutex<PromptRenderState>>) -> Self {
        Self { state }
    }

    fn prompt_len(&self) -> usize {
        "scopeql".len()
    }

    fn prompt_indicator(&self) -> String {
        let state = self.state.lock().unwrap();
        if state.input_is_empty() {
            format!("> \x1b[s\n{}\x1b[u", state.line())
        } else {
            "> ".to_string()
        }
    }
}

impl Prompt for CommandLinePrompt {
    fn render_prompt_left(&'_ self) -> Cow<'_, str> {
        "scopeql".into()
    }

    fn render_prompt_right(&'_ self) -> Cow<'_, str> {
        "".into()
    }

    fn render_prompt_indicator(&'_ self, _: PromptEditMode) -> Cow<'_, str> {
        self.prompt_indicator().into()
    }

    fn render_prompt_multiline_indicator(&'_ self) -> Cow<'_, str> {
        format!("{:width$}> ", " ", width = self.prompt_len()).into()
    }

    fn render_prompt_history_search_indicator(
        &'_ self,
        history_search: PromptHistorySearch,
    ) -> Cow<'_, str> {
        // NOTE: This is copied from the DefaultPrompt implementation.
        let PromptHistorySearch { term, status } = history_search;
        let prefix = match status {
            PromptHistorySearchStatus::Passing => "",
            PromptHistorySearchStatus::Failing => "failing ",
        };
        Cow::Owned(format!("({prefix}reverse-search: {term}) "))
    }

    fn get_prompt_color(&self) -> reedline::Color {
        reedline::Color::DarkGrey
    }

    fn get_indicator_color(&self) -> reedline::Color {
        reedline::Color::DarkGrey
    }

    fn get_prompt_multiline_color(&self) -> nu_ansi_term::Color {
        nu_ansi_term::Color::DarkGray
    }
}

pub struct StatusHinter {
    inner: DefaultHinter,
    state: Arc<Mutex<PromptRenderState>>,
    status_style: Style,
}

impl StatusHinter {
    pub fn new(inner: DefaultHinter, state: Arc<Mutex<PromptRenderState>>) -> Self {
        Self {
            inner,
            state,
            status_style: Style::new().fg(nu_ansi_term::Color::DarkGray),
        }
    }

    fn render_status(&self, use_ansi_coloring: bool) -> String {
        let status = self.state.lock().unwrap().line().to_string();

        if use_ansi_coloring {
            self.status_style.paint(status).to_string()
        } else {
            status
        }
    }

    fn render_status_only_hint(status: &str) -> String {
        // When DefaultHinter has no history suffix, the status bar is the whole
        // hint. Reedline does not reliably render a hint beginning with a
        // newline, so prefix a non-printing SGR reset.
        format!("\x1b[0m\n{status}")
    }
}

impl Hinter for StatusHinter {
    fn handle(
        &mut self,
        line: &str,
        pos: usize,
        history: &dyn History,
        use_ansi_coloring: bool,
        cwd: &str,
    ) -> String {
        let hint = self
            .inner
            .handle(line, pos, history, use_ansi_coloring, cwd);
        if line.is_empty() {
            self.state.lock().unwrap().set_input_is_empty(true);
            return hint;
        }

        self.state.lock().unwrap().set_input_is_empty(false);

        let status = self.render_status(use_ansi_coloring);

        if hint.is_empty() {
            Self::render_status_only_hint(&status)
        } else {
            format!("{hint}\n{status}")
        }
    }

    fn complete_hint(&self) -> String {
        self.inner.complete_hint()
    }

    fn next_hint_token(&self) -> String {
        self.inner.next_hint_token()
    }
}
