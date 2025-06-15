pub trait LabelIter {
    type LabelKey: AsRef<str>;
    type LabelVal: AsRef<str>;

    fn next(&mut self) -> Option<(Self::LabelKey, Self::LabelVal)>;
    fn iter(mut self) -> impl Iterator<Item = (Self::LabelKey, Self::LabelVal)>
    where
        Self: Sized,
    {
        std::iter::from_fn(move || self.next())
    }
}

/// Use this when NoLabels are present
pub struct NoLabels;

impl LabelIter for NoLabels {
    type LabelKey = &'static str;
    type LabelVal = &'static str;

    fn next(&mut self) -> Option<(Self::LabelKey, Self::LabelVal)> {
        None
    }
}

impl<ITER, LKey, LVal> LabelIter for ITER
where
    ITER: Iterator<Item = (LKey, LVal)>,
    LKey: AsRef<str>,
    LVal: AsRef<str>,
{
    type LabelKey = LKey;
    type LabelVal = LVal;

    fn next(&mut self) -> Option<(Self::LabelKey, Self::LabelVal)> {
        self.next()
    }
}
