use crate::{
    BorrowWindow, Bounds, Element, LayoutId, Pixels, Result, SharedString, Style, StyleHelpers,
    Styled, ViewContext,
};
use futures::FutureExt;
use refineable::RefinementCascade;
use std::marker::PhantomData;
use util::ResultExt;

pub struct Img<S> {
    style: RefinementCascade<Style>,
    uri: Option<SharedString>,
    grayscale: bool,
    state_type: PhantomData<S>,
}

pub fn img<S>() -> Img<S> {
    Img {
        style: RefinementCascade::default(),
        uri: None,
        grayscale: false,
        state_type: PhantomData,
    }
}

impl<S> Img<S> {
    pub fn uri(mut self, uri: impl Into<SharedString>) -> Self {
        self.uri = Some(uri.into());
        self
    }

    pub fn grayscale(mut self, grayscale: bool) -> Self {
        self.grayscale = grayscale;
        self
    }
}

impl<S: Send + Sync + 'static> Element for Img<S> {
    type State = S;
    type FrameState = ();

    fn layout(
        &mut self,
        _: &mut Self::State,
        cx: &mut ViewContext<Self::State>,
    ) -> anyhow::Result<(LayoutId, Self::FrameState)>
    where
        Self: Sized,
    {
        let style = self.computed_style();
        let layout_id = cx.request_layout(style, [])?;
        Ok((layout_id, ()))
    }

    fn paint(
        &mut self,
        bounds: Bounds<Pixels>,
        _: &mut Self::State,
        _: &mut Self::FrameState,
        cx: &mut ViewContext<Self::State>,
    ) -> Result<()> {
        let style = self.computed_style();

        style.paint(bounds, cx);

        if let Some(uri) = self.uri.clone() {
            let image_future = cx.image_cache.get(uri);
            if let Some(data) = image_future
                .clone()
                .now_or_never()
                .and_then(ResultExt::log_err)
            {
                let corner_radii = style.corner_radii.to_pixels(bounds.size, cx.rem_size());
                cx.stack(1, |cx| {
                    cx.paint_image(bounds, corner_radii, data, self.grayscale)
                })?;
            } else {
                cx.spawn(|_, mut cx| async move {
                    if image_future.await.log_err().is_some() {
                        cx.on_next_frame(|cx| cx.notify());
                    }
                })
                .detach()
            }
        }
        Ok(())
    }
}

impl<S> Styled for Img<S> {
    type Style = Style;

    fn style_cascade(&mut self) -> &mut RefinementCascade<Self::Style> {
        &mut self.style
    }

    fn declared_style(&mut self) -> &mut <Self::Style as refineable::Refineable>::Refinement {
        self.style.base()
    }
}

impl<S> StyleHelpers for Img<S> {}
