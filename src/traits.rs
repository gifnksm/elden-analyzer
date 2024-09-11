use num_rational::Ratio;

pub trait ToRatio {
    fn to_ratio(&self) -> Ratio<i64>;
}

impl ToRatio for ffmpeg::Rational {
    fn to_ratio(&self) -> Ratio<i64> {
        Ratio::new(self.numerator().into(), self.denominator().into())
    }
}

impl ToRatio for ffmpeg::ffi::AVRational {
    fn to_ratio(&self) -> Ratio<i64> {
        ffmpeg::Rational::from(*self).to_ratio()
    }
}
