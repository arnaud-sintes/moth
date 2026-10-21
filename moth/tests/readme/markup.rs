use moth::{Declarative, ElementRef, Label, Slider};

fn main() {
	// make the stock elements work in markup; those already in the page are upgraded
	moth::define_all().unwrap();

	// typed access to what the page declared (the DOM owns these: it ends with the element)
	let title = Label::of(ElementRef::by_id("title").unwrap().element()).unwrap();
	let volume = ElementRef::by_id("volume").unwrap().declared::<Slider>().unwrap();
	title.with(|title| title.set_text("Sound settings"));

	// behaviour for an element that is already there, gone when its owner drops the listener
	let reset = ElementRef::by_id("reset").unwrap();
	let click = reset
		.on("click", move |_| {
			volume.with(|slider| slider.set_value(3.0));
		})
		.unwrap();
	moth::mount(click).unwrap();
}
