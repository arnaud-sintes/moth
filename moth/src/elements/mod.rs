mod body;
mod canvas;
mod dropdown;
mod edit;
mod image;
mod label;
mod panel;
mod progress_bar;
mod push_button;
mod slider;
mod toggle_button;

pub use body::Body;
pub use canvas::{Canvas, CanvasHandle, Size};
pub use dropdown::Dropdown;
pub use edit::Edit;
pub use image::Image;
pub use label::Label;
pub use panel::Panel;
pub use progress_bar::{ProgressBar, ProgressHandle};
pub use push_button::PushButton;
pub use slider::{Slider, SliderHandle};
pub use toggle_button::ToggleButton;

use std::rc::Rc;

use web_sys::HtmlButtonElement;

use crate::{Error, Host, Prop, component::set_form_value_on};

/// Wire the shadow `<button>` of a button-like element to the host's effective `enabled` state.
fn attach_button(host: &Host) -> Result<HtmlButtonElement, Error> {
	let button: HtmlButtonElement = host.shadow("button")?;
	let inner = button.clone();
	host.effective.bind(move |effective| inner.set_disabled(!effective));
	Ok(button)
}

/// Keep what a form control submits in sync: `render(value)` while it is effectively enabled,
/// nothing while it or an ancestor is disabled. (A form-associated element is only left out of a
/// submission by the browser when *its own* `disabled` attribute is set, which an inherited
/// disabled state does not do.)
fn bind_form_value<T: Clone + PartialEq + 'static>(
	host: &Host,
	value: &Prop<T>,
	render: impl Fn(&T) -> Option<String> + 'static,
) {
	let render = Rc::new(render);
	let (element, effective) = (host.element().clone(), host.effective.downgrade());
	let on_value = render.clone();
	value.bind(move |value| {
		let enabled = effective.get().unwrap_or(true);
		set_form_value_on(&element, if enabled { on_value(value) } else { None });
	});

	let (element, current) = (host.element().clone(), value.downgrade());
	host.effective.bind(move |enabled| {
		if let Some(value) = current.get() {
			set_form_value_on(&element, if *enabled { render(&value) } else { None });
		}
	});
}
