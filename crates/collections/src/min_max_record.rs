use std::fmt;

#[derive(Debug)]
struct Inner<K, V> {
    min_key: K,
    min_value: V,
    max_key: K,
    max_value: V,
}

impl<K, V> Inner<K, V> {
    fn new(key: K, value: V) -> Self
    where
        K: Clone,
        V: Clone,
    {
        Self {
            min_key: key.clone(),
            min_value: value.clone(),
            max_key: key,
            max_value: value,
        }
    }
}

#[derive(Debug)]
pub struct MinMaxRecord<K, V> {
    inner: Option<Inner<K, V>>,
}

impl<K, V> Default for MinMaxRecord<K, V> {
    fn default() -> Self {
        Self { inner: None }
    }
}

impl<K, V> MinMaxRecord<K, V> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        self.inner = None;
    }

    pub fn update(&mut self, key: K, value: V)
    where
        K: Clone,
        V: Ord + Clone,
    {
        match &mut self.inner {
            None => {
                self.inner = Some(Inner::new(key, value));
            }
            Some(Inner {
                min_key,
                min_value,
                max_key,
                max_value,
            }) => {
                assert!(min_value <= max_value);
                if value < *min_value {
                    assert!(value < *max_value);
                    *min_key = key;
                    *min_value = value;
                } else if value > *max_value {
                    assert!(value > *min_value);
                    *max_key = key;
                    *max_value = value;
                }
            }
        }
    }

    pub fn get_min(&self) -> Option<(&K, &V)> {
        let Inner {
            min_key, min_value, ..
        } = self.inner.as_ref()?;
        Some((min_key, min_value))
    }

    pub fn get_max(&self) -> Option<(&K, &V)> {
        let Inner {
            max_key, max_value, ..
        } = self.inner.as_ref()?;
        Some((max_key, max_value))
    }

    pub fn append(&mut self, other: &mut Self)
    where
        V: Ord,
    {
        match (&mut self.inner, other.inner.take()) {
            (_, None) => {}
            (None, Some(inner)) => {
                self.inner = Some(inner);
            }
            (Some(this), Some(other)) => {
                if other.min_value < this.min_value {
                    this.min_key = other.min_key;
                    this.min_value = other.min_value;
                }
                if other.max_value > this.max_value {
                    this.max_key = other.max_key;
                    this.max_value = other.max_value;
                }
            }
        }
    }
}

impl<K, V> fmt::Display for MinMaxRecord<K, V>
where
    K: fmt::Display,
    V: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.inner {
            Some(Inner {
                min_key,
                min_value,
                max_key,
                max_value,
            }) => write!(
                f,
                "{{min:{}={}, max:{}={}}}",
                min_key, min_value, max_key, max_value
            ),
            None => write!(f, "None"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic() {
        let mut record = MinMaxRecord::new();
        assert!(record.get_min().is_none());
        assert!(record.get_max().is_none());

        record.update(1, 10);
        assert_eq!(record.get_min(), Some((&1, &10)));
        assert_eq!(record.get_max(), Some((&1, &10)));

        record.update(2, 20);
        assert_eq!(record.get_min(), Some((&1, &10)));
        assert_eq!(record.get_max(), Some((&2, &20)));

        record.update(3, 0);
        assert_eq!(record.get_min(), Some((&3, &0)));
        assert_eq!(record.get_max(), Some((&2, &20)));

        record.update(4, 10);
        assert_eq!(record.get_min(), Some((&3, &0)));
        assert_eq!(record.get_max(), Some((&2, &20)));

        record.update(5, 20);
        assert_eq!(record.get_min(), Some((&3, &0)));
        assert_eq!(record.get_max(), Some((&2, &20)));
    }
}
