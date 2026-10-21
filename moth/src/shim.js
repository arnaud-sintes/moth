// The only hand-written JavaScript in moth: wasm-bindgen cannot define a class that extends HTMLElement.
// Generic by design: adding a component never requires touching this file.

let hook = null;

// Rust registers one function receiving every custom element lifecycle event:
// hook(kind, element, attributeName, attributeValue), kind in "connected" | "disconnected" | "attribute".
export function set_hook(callback) {
	hook = callback;
}

export function define_element(tag, templateHtml, css, attributes, formAssociated) {
	const existing = customElements.get(tag);
	if (existing) {
		// idempotent for moth's own classes; anything else under that tag would silently get the wrong
		// template, attributes and behaviour
		if (!existing.__moth) throw new Error(`<${tag}> is already defined by something other than moth`);
		return;
	}
	const template = document.createElement("template");
	template.innerHTML = templateHtml;
	const sheet = new CSSStyleSheet();
	sheet.replaceSync(css);
	customElements.define(tag, class extends HTMLElement {
		static __moth = true;
		static formAssociated = formAssociated;
		static observedAttributes = attributes;

		constructor() {
			super();
			this.__moth_internals = this.attachInternals();
			// delegatesFocus: focusing the host (or a click on its padding) focuses the inner control
			const root = this.attachShadow({ mode: "open", delegatesFocus: true });
			root.adoptedStyleSheets = [sheet];
			root.append(template.content.cloneNode(true));
		}

		connectedCallback() {
			// the ancestors may differ from last time (first connection, or a move): the component
			// recomputes whether it is enabled
			this.dispatchEvent(new Event("moth-enabled-changed"));
			if (hook) hook("connected", this, null, null);
		}

		disconnectedCallback() {
			// moving an element disconnects then reconnects it within one task: only report a
			// disconnection that is still true once the task is over
			queueMicrotask(() => {
				if (!this.isConnected && hook) hook("disconnected", this, null, null);
			});
		}

		attributeChangedCallback(name, _previous, value) {
			// the own `disabled` state is the attribute, whoever writes it (Rust, markup or a script)
			if (name === "disabled") this.dispatchEvent(new Event("moth-disabled-attribute"));
			if (hook) hook("attribute", this, name, value);
		}
	});
}

// Value submitted with the enclosing <form> (only for form-associated elements); null submits nothing.
export function set_form_value(element, value) {
	element.__moth_internals.setFormValue(value);
}
