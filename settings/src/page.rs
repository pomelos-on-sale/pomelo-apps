//! A page, tagged by the section it shows.
//!
//! One requirement of the port is that switching pages opens the new page at the top. iced keeps
//! a `Scrollable`'s offset in the widget tree, in the node that scrollable owns, and that node
//! survives a rebuild for as long as the widget's [`Widget::tag`] is unchanged — so the offset
//! would survive a section change too, and a sub-page would open wherever the previous one had
//! been scrolled to. There is no "reset the scroll" operation this stack can run: an operation
//! travels as an iced `Task`, and this backend has no executor.
//!
//! So the reset is structural. [`Page`] is a transparent wrapper whose tag is a marker type, one
//! per section, so a different section is a different node in the tree: the page below it — the
//! scrollable included — is built from scratch, at offset zero. Re-rendering the *same* section
//! keeps the tag, and therefore keeps the offset, which is also what the original
//! did (its `set_section` only reset the scroll when the section actually changed).
//!
//! Everything but the tag forwards to the wrapped page, the way `iced_widget`'s own transparent
//! wrappers (`Float`, `Pin`) do: same state, same children, same layout node. There is nothing
//! else to reset and nothing extra to lay out.

use std::marker::PhantomData;

use iced::advanced::widget::{tree, Operation, Tree};
use iced::advanced::{layout, mouse, overlay, renderer, Clipboard, Layout, Shell, Widget};
use iced::{Element, Event, Length, Rectangle, Size, Vector};

/// Marks the main list page.
pub struct MainTag;
/// Marks the Wi-Fi page.
pub struct WifiTag;
/// Marks the memory page.
pub struct MemoryTag;
/// Marks the storage page.
pub struct StorageTag;
/// Marks the battery page.
pub struct BatteryTag;
/// Marks the system-info page.
pub struct SystemTag;
/// Marks the theme page.
pub struct ThemeTag;
/// Marks the date-and-time page.
pub struct TimeTag;

/// Wraps a page so that its widget-tree node is per-section.
///
/// `Tag` is the marker above, and it is the *whole* reason this type exists: see the module
/// documentation. The state, children and layout are the wrapped page's, unchanged.
pub struct Page<'a, Tag, Message, Theme, Renderer>
where
    Renderer: iced::advanced::Renderer,
{
    content: Element<'a, Message, Theme, Renderer>,
    marker: PhantomData<Tag>,
}

impl<'a, Tag, Message, Theme, Renderer> Page<'a, Tag, Message, Theme, Renderer>
where
    Renderer: iced::advanced::Renderer,
{
    /// Wraps `content` under the section marker `Tag`.
    pub fn new(content: impl Into<Element<'a, Message, Theme, Renderer>>) -> Self {
        Self {
            content: content.into(),
            marker: PhantomData,
        }
    }
}

/// Wraps a page under the section marker `Tag`.
///
/// The free-function spelling, because a `Page<'_, Tag, ..>` in a `match` arm would otherwise
/// need its lifetime written out at every call site.
pub fn section_page<'a, Tag, Message, Theme, Renderer>(
    content: impl Into<Element<'a, Message, Theme, Renderer>>,
) -> Element<'a, Message, Theme, Renderer>
where
    Tag: 'static,
    Message: 'a,
    Theme: 'a,
    Renderer: iced::advanced::Renderer + 'a,
{
    Page::<'_, Tag, Message, Theme, Renderer>::new(content).into()
}

impl<Tag, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for Page<'_, Tag, Message, Theme, Renderer>
where
    Tag: 'static,
    Renderer: iced::advanced::Renderer,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<Tag>()
    }

    fn state(&self) -> tree::State {
        self.content.as_widget().state()
    }

    fn children(&self) -> Vec<Tree> {
        self.content.as_widget().children()
    }

    fn diff(&self, tree: &mut Tree) {
        self.content.as_widget().diff(tree);
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content.as_widget_mut().layout(tree, renderer, limits)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        self.content
            .as_widget_mut()
            .operate(tree, layout, renderer, operation);
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        self.content.as_widget_mut().update(
            tree, event, layout, cursor, renderer, clipboard, shell, viewport,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.content
            .as_widget()
            .mouse_interaction(tree, layout, cursor, viewport, renderer)
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.content
            .as_widget()
            .draw(tree, renderer, theme, style, layout, cursor, viewport);
    }

    fn overlay<'a>(
        &'a mut self,
        tree: &'a mut Tree,
        layout: Layout<'a>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'a, Message, Theme, Renderer>> {
        self.content
            .as_widget_mut()
            .overlay(tree, layout, renderer, viewport, translation)
    }
}

impl<'a, Tag, Message, Theme, Renderer> From<Page<'a, Tag, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Tag: 'static,
    Message: 'a,
    Theme: 'a,
    Renderer: iced::advanced::Renderer + 'a,
{
    fn from(page: Page<'a, Tag, Message, Theme, Renderer>) -> Self {
        Element::new(page)
    }
}
