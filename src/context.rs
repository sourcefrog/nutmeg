// Copyright 2023 Martin Pool.

//! Context passed from the library to the render function.

pub struct RenderContext {
    pub(crate) width: usize,
}

impl RenderContext {
    /// Return the width of the terminal to which this text is being rendered.
    ///
    /// If the size can't be determined this function returns a default size.
    ///
    /// The [Model::render] implementation may make us of this to, for
    /// example, draw a full-width progress bar, or to selectively truncate
    /// sections within the line.
    ///
    /// The model may also ignore the width and return a string
    /// of any width, in which case it will be truncated to fit on the
    /// screen.
    pub fn width(&self) -> usize {
        self.width
    }
}
