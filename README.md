# nutmeg

Manage a console/terminal UI that can alternate between showing a progress
bar and lines of text output.

### Concept

By contrast to other Rust progress-bar libraries, Nutmeg has no
built-in concept of what the progress bar or indicator should look like:
this is entirely under the control of the application. Nutmeg handles
drawing the application's progress bar to the screen and removing it as needed.

The application (or dependent library) is responsible for:

* Defining a type that implements [State], which holds whatever information
  is relevant to drawing progress.
* Defining how to render that information into some text lines, by
  implementing [State::render].
* Constructing a [View] that will draw progress to the terminal.
* Notifying the [View] when there are state updates, by calling
  [View::update].
* While a [View] is in use, all text written to stdout/stderr should be sent
  via that view, to avoid the display getting scrambled.

The Nutmeg library is responsible for:

* Periodically drawing the progress bar in response to updates, including
  * Horizontally truncating output to fit on the screen.
  * Handling changes in the number of lines of progress display.
* Removing the progress bar when the view is finished or dropped.
* Coordinating to hide the bar to print text output, and restore it
  afterwards.
* Limiting the rate at which updates are drawn to the screen.

Errors in writing to the terminal cause a panic.

### Potential future features

* Draw updates from a background thread, so that it will keep ticking even
  if not actively updated, and to better handle applications that send a
  burst of updates followed by a long pause. The background thread will
  eventually paint the last drawn update.

License: MIT
