use wasm_bindgen::JsCast;
use web_sys::HtmlSelectElement;
use yew::prelude::*;

pub fn time_signature_options(values: &[u32]) -> Vec<Html> {
    values
        .iter()
        .map(|x| {
            html! {
                <option value={ x.to_string() } selected={ x == &4 }>
                    { x.to_string() }
                </option>
            }
        })
        .collect()
}

pub fn select_get_value(event: Event) -> Option<String> {
    let target = event.target();
    let select = target.and_then(|target| target.dyn_into::<HtmlSelectElement>().ok());

    select.and_then(|select| Some(select.value()))
}
