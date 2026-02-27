from textual import work
from textual.app import App, ComposeResult
from textual.binding import Binding
from textual.containers import Horizontal, Vertical, VerticalScroll
from textual.screen import ModalScreen
from textual.widgets import (
    Button,
    ContentSwitcher,
    Footer,
    Input,
    Label,
    ListItem,
    ListView,
    LoadingIndicator,
    Static,
    TextArea,
)
import treesearch
import asyncio
from pathlib import Path


class DetailPane(Vertical):
    """Pane to show tree and match details."""

    DEFAULT_CSS = """
    DetailPane {
        display: none;
        height: 0;
        background: $surface;
        border-top: thick $primary;
    }

    DetailPane.visible {
        display: block;
        height: 50%;
    }

    #detail-scroll {
        height: 1fr;
    }

    #detail-content {
        padding: 1 2;
    }
    """

    def __init__(self):
        super().__init__()
        self._tree = None
        self._match = None

    def compose(self) -> ComposeResult:
        with VerticalScroll(id="detail-scroll"):
            yield Static("", id="detail-content")

    def show_details(self, tree, match):
        """Update and show the detail pane."""
        self._tree = tree
        self._match = match
        content = self.query_one("#detail-content", Static)
        content.update(self._format_details())
        self.add_class("visible")
        self.scroll_home()

    def hide_details(self):
        """Hide the detail pane."""
        self.remove_class("visible")

    def _format_details(self) -> str:
        """Format tree and match details for display."""
        if not self._tree or not self._match:
            return ""

        lines = []

        # Show full sentence
        if self._tree.sentence_text is not None:
            lines.append("[bold]Sentence:[/bold]")
            lines.append(self._tree.sentence_text)
            lines.append("")

        # Show match variables
        lines.append("[bold]Matched words:[/bold]")
        for var_name, word_id in self._match.items():
            word = self._tree.word(word_id)
            lines.append(f"  {var_name}: {word.form} (lemma={word.lemma}, upos={word.upos})")
        lines.append("")

        # Show metadata
        if self._tree.metadata:
            lines.append("[bold]Metadata:[/bold]")
            for key, value in self._tree.metadata.items():
                lines.append(f"  {key}: {value}")
            lines.append("")

        # Show full tree structure
        lines.append("[bold]Tree structure:[/bold]")
        for i in range(len(self._tree)):
            word = self._tree.word(i)
            head = str(word.head) if word.head is not None else "ROOT"

            if i in self._match.values():
                line = f"[reverse]{word.id:3} {word.form:15} {word.lemma:15} {word.upos:10} {word.deprel:10} {head:4}[/reverse]"
            else:
                line = f"{word.id:3} {word.form:15} {word.lemma:15} {word.upos:10} {word.deprel:10} {head:4}"

            lines.append(line)

        return "\n".join(lines)


class FileModal(ModalScreen[str | None]):
    """Modal dialog for file load/save path entry."""

    DEFAULT_CSS = """
    FileModal {
        align: center middle;
    }

    FileModal > Vertical {
        width: 60;
        height: auto;
        background: $surface;
        border: thick $primary;
        padding: 1 2;
    }

    FileModal Label {
        width: 1fr;
        margin-bottom: 1;
    }

    FileModal Input {
        width: 1fr;
        margin-bottom: 1;
    }

    FileModal Horizontal {
        width: 1fr;
        height: auto;
        align-horizontal: right;
    }

    FileModal Button {
        margin-left: 1;
    }
    """

    BINDINGS = [Binding("escape", "dismiss(None)", "Cancel")]

    def __init__(self, prompt: str):
        super().__init__()
        self._prompt = prompt

    def compose(self) -> ComposeResult:
        with Vertical():
            yield Label(self._prompt)
            yield Input(placeholder="file path...", id="file-modal-input")
            with Horizontal():
                yield Button("OK", id="ok", variant="primary")
                yield Button("Cancel", id="cancel")

    def on_mount(self) -> None:
        self.query_one("#file-modal-input", Input).focus()

    def on_button_pressed(self, event: Button.Pressed) -> None:
        if event.button.id == "ok":
            self.dismiss(self.query_one("#file-modal-input", Input).value.strip())
        else:
            self.dismiss(None)

    def on_input_submitted(self, event: Input.Submitted) -> None:
        self.dismiss(event.value.strip())


class TermconcApp(App):
    CSS = """
    Static {
        padding: 0;
        border-bottom: solid #444;
        text-wrap: wrap;
    }

    ListView {
        height: 1fr;
    }

    ContentSwitcher {
        height: 1fr;
    }

    #concordance-view {
        height: 1fr;
    }

    #query-editor {
        height: 1fr;
    }
    """

    BINDINGS = [
        Binding("ctrl+e", "show_editor", "Edit query"),
        Binding("ctrl+r", "run_query", "Run query"),
        Binding("ctrl+o", "load_query", "Load query"),
        Binding("ctrl+s", "save_query", "Save query"),
        Binding("escape", "back", "Back"),
    ]

    def __init__(self):
        super().__init__()
        self.detail_visible = False
        self.current_query = ""
        self.pattern = None
        self.item_data = {}

    def compose(self) -> ComposeResult:
        with ContentSwitcher(initial="query-editor"):
            with Vertical(id="concordance-view"):
                yield ListView(id="result-list")
                yield DetailPane()
            yield TextArea("", id="query-editor")
        yield Footer()

    def on_mount(self):
        path = "/Volumes/Corpora/NA_NEWS/parsed/latwp/**/*.conll.gz"
        self.treebank = treesearch.load(path)
        self.query_one("#query-editor", TextArea).focus()

    def check_action(self, action: str, parameters: tuple) -> bool | None:
        switcher = self.query_one(ContentSwitcher)
        in_editor = switcher.current == "query-editor"
        if action == "show_editor":
            return not in_editor
        if action == "run_query":
            return in_editor
        if action == "back":
            return in_editor or self.detail_visible
        return True

    def on_list_view_selected(self, event: ListView.Selected) -> None:
        index = event.list_view.index
        if index is not None and index in self.item_data:
            tree, match = self.item_data[index]
            self.query_one(DetailPane).show_details(tree, match)
            self.detail_visible = True
            self.refresh_bindings()

    def action_show_editor(self) -> None:
        self.query_one(ContentSwitcher).current = "query-editor"
        self.query_one("#query-editor", TextArea).focus()
        self.refresh_bindings()

    def action_run_query(self) -> None:
        new_query = self.query_one("#query-editor", TextArea).text
        try:
            self.pattern = treesearch.compile_query(new_query)
        except ValueError as e:
            self.notify(str(e), severity="error")
            return
        self.current_query = new_query
        self.query_one(ContentSwitcher).current = "concordance-view"
        self.refresh_bindings()
        self._reload_results()

    def action_load_query(self) -> None:
        def handle_path(path: str | None) -> None:
            if not path:
                return
            try:
                text = Path(path).read_text()
            except OSError as e:
                self.notify(str(e), severity="error")
                return
            try:
                self.pattern = treesearch.compile_query(text)
            except ValueError as e:
                self.notify(str(e), severity="error")
                return
            self.query_one("#query-editor", TextArea).load_text(text)
            self.current_query = text
            self.query_one(ContentSwitcher).current = "concordance-view"
            self.refresh_bindings()
            self._reload_results()
            self.notify(f"Loaded {path}")

        self.push_screen(FileModal("Load query:"), handle_path)

    def action_save_query(self) -> None:
        def handle_path(path: str | None) -> None:
            if not path:
                return
            query = self.query_one("#query-editor", TextArea).text
            try:
                Path(path).write_text(query)
                self.notify(f"Saved {path}")
            except OSError as e:
                self.notify(str(e), severity="error")

        self.push_screen(FileModal("Save query:"), handle_path)

    def action_back(self) -> None:
        switcher = self.query_one(ContentSwitcher)
        if switcher.current == "query-editor":
            switcher.current = "concordance-view"
            self.refresh_bindings()
        elif self.detail_visible:
            self.query_one(DetailPane).hide_details()
            self.detail_visible = False
            self.refresh_bindings()

    def _reload_results(self) -> None:
        self.query_one("#result-list", ListView).clear()
        self.item_data = {}
        if self.detail_visible:
            self.query_one(DetailPane).hide_details()
            self.detail_visible = False
        self.overlay = LoadingIndicator()
        self.mount(self.overlay)
        self.load_data_background()

    @work(exclusive=True)
    async def load_data_background(self):
        lv = self.query_one("#result-list", ListView)
        batch = []
        item_index = 0

        for i, (tree, match) in enumerate(self.treebank.search(self.pattern, ordered=True)):
            matches = match.values()
            text = []
            for j in range(len(tree)):
                if j in matches:
                    text.append(f"[orange bold]{tree[j].form}[/orange bold]")
                else:
                    text.append(tree[j].form)
            text = " ".join(text)
            styled_row = Static(text)
            batch.append(ListItem(styled_row))

            self.item_data[item_index] = (tree, match)
            item_index += 1

            if i > 500:
                break
            if len(batch) > 100:
                await lv.extend(batch)
                batch = []
                await asyncio.sleep(0)  # Yield to allow UI updates

        if batch:
            await lv.extend(batch)

        await self.overlay.remove()


if __name__ == "__main__":
    TermconcApp().run()
