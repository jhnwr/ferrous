use scraper::{ElementRef, Html, Selector};

pub struct Element {
    html: String,
}

impl Element {
    pub fn from_element_ref(el: ElementRef<'_>) -> Self {
        Self {
            html: el.html(),
        }
    }

    fn doc(&self) -> Html {
        Html::parse_fragment(&self.html)
    }

    /// Find the first real element in the fragment (skips the synthetic html wrapper).
    fn root_el<'a>(doc: &'a Html) -> Option<ElementRef<'a>> {
        // parse_fragment wraps content in a synthetic <html> root node.
        // The actual element is a direct child of that root.
        doc.root_element()
            .children()
            .find_map(ElementRef::wrap)
    }

    /// Text content of the matched element itself (direct and descendant text nodes).
    pub fn text(&self) -> Option<String> {
        let doc = self.doc();
        let el = Self::root_el(&doc)?;
        let text: String = el.text().collect::<Vec<_>>().join("").trim().to_string();
        if text.is_empty() { None } else { Some(text) }
    }

    /// Attribute value on the matched element itself.
    pub fn attr(&self, name: &str) -> Option<String> {
        let doc = self.doc();
        let el = Self::root_el(&doc)?;
        el.value().attr(name).map(|s| s.to_string())
    }

    /// Text content of the first child element matching `selector`.
    pub fn select_text(&self, selector: &str) -> Option<String> {
        let sel = Selector::parse(selector).ok()?;
        let doc = self.doc();
        let el = doc.select(&sel).next()?;
        let text: String = el.text().collect::<Vec<_>>().join("").trim().to_string();
        if text.is_empty() { None } else { Some(text) }
    }

    /// Attribute value of the first child element matching `selector`.
    pub fn select_attr(&self, selector: &str, attr: &str) -> Option<String> {
        let sel = Selector::parse(selector).ok()?;
        let doc = self.doc();
        let el = doc.select(&sel).next()?;
        el.value().attr(attr).map(|s| s.to_string())
    }
}
