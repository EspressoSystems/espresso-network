use serde::{Deserialize, Serialize};

/// [LocationDetails] represents the details of the location of the node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LocationDetails {
    pub coords: Option<(f64, f64)>,
    pub country: Option<String>,
}

impl LocationDetails {
    pub fn new(coords: Option<(f64, f64)>, country: Option<String>) -> Self {
        Self { coords, country }
    }

    pub fn coords(&self) -> &Option<(f64, f64)> {
        &self.coords
    }

    pub fn country(&self) -> &Option<String> {
        &self.country
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_location_details_coords() {
        let coords = (0.0, 0.0);
        let country = "US".to_string();
        let location_details = LocationDetails::new(Some(coords), Some(country.clone()));

        assert_eq!(location_details.coords(), &Some(coords));
    }

    #[test]
    fn test_location_details_country() {
        let coords = (0.0, 0.0);
        let country = "US".to_string();
        let location_details = LocationDetails::new(Some(coords), Some(country.clone()));

        assert_eq!(location_details.country(), &Some(country));
    }

    #[test]
    fn test_location_details_eq() {
        let coords = (0.0, 0.0);
        let country = "US".to_string();
        let location_details = LocationDetails::new(Some(coords), Some(country.clone()));
        let location_details_2 = LocationDetails::new(Some(coords), Some(country.clone()));

        assert_eq!(location_details, location_details_2);
    }

    #[test]
    fn test_location_details_debug() {
        let coords = (0.0, 0.0);
        let country = "US".to_string();
        let location_details = LocationDetails::new(Some(coords), Some(country.clone()));

        assert_eq!(
            format!("{location_details:?}"),
            format!(
                "LocationDetails {{ coords: Some({:?}), country: Some({:?}) }}",
                coords, country
            )
        );
    }

    #[test]
    fn test_location_details_clone() {
        let coords = (0.0, 0.0);
        let country = "US".to_string();
        let location_details = LocationDetails::new(Some(coords), Some(country.clone()));
        let cloned_location_details = location_details.clone();

        assert_eq!(location_details, cloned_location_details);
    }

    #[test]
    #[cfg(feature = "testing")]
    fn test_location_serialization() {
        use serde_json;

        let coords = (1.2, 3.4);
        let country = "US".to_string();
        let location_details = LocationDetails::new(Some(coords), Some(country.clone()));

        let serialized = serde_json::to_string(&location_details).unwrap();
        let deserialized: LocationDetails = serde_json::from_str(&serialized).unwrap();

        assert_eq!(location_details, deserialized);
    }
}
