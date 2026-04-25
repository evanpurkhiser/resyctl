use serde_json::{Value, json};

use crate::api::ResyClient;
use crate::cli::VenueArgs;
use crate::error::Error;
use crate::models::VenueLocation;
use crate::util::to_json_value;

pub async fn run(client: &ResyClient, args: VenueArgs) -> Result<Value, Error> {
    let venue = client.venue(args.venue_id).await?;
    let raw = to_json_value(&venue)?;

    let address = venue.location.as_ref().map(format_address);
    let coordinates = venue
        .location
        .as_ref()
        .and_then(|l| Some((l.latitude?, l.longitude?)))
        .map(|(lat, lng)| json!({ "latitude": lat, "longitude": lng }));
    let google_maps_url = venue
        .location
        .as_ref()
        .and_then(|loc| google_maps_url(venue.name.as_deref(), loc));

    let content_lookup = |name: &str| -> Option<String> {
        venue
            .content
            .iter()
            .find(|c| c.name.as_deref() == Some(name))
            .and_then(|c| c.body.clone())
            .filter(|s| !s.trim().is_empty())
    };

    let socials: serde_json::Map<String, Value> = venue
        .social
        .iter()
        .filter_map(|s| Some((s.name.clone()?, Value::String(s.value.clone()?))))
        .collect();

    Ok(json!({
        "ok": true,
        "venue": {
            "id": venue.id.as_ref().and_then(|id| id.resy),
            "name": venue.name,
            "type": venue.kind,
            "neighborhood": venue.location.as_ref().and_then(|l| l.neighborhood.clone()),
            "address": address,
            "coordinates": coordinates,
            "phone": venue.contact.as_ref().and_then(|c| c.phone_number.clone()),
            "website": venue.contact.as_ref().and_then(|c| c.url.clone()),
            "menu_url": venue.contact.as_ref().and_then(|c| c.menu_url.clone()),
            "resy_url": venue.links.as_ref().and_then(|l| l.web.clone()),
            "google_maps_url": google_maps_url,
            "description": venue.metadata.as_ref().and_then(|m| m.description.clone()),
            "tagline": content_lookup("tagline"),
            "about": content_lookup("about"),
            "need_to_know": content_lookup("need_to_know"),
            "from_the_venue": content_lookup("from_the_venue"),
            "social": socials,
        },
        "raw": raw,
    }))
}

fn format_address(loc: &VenueLocation) -> String {
    let mut parts: Vec<&str> = Vec::new();
    if let Some(s) = loc.address_1.as_deref().filter(|s| !s.is_empty()) {
        parts.push(s);
    }
    if let Some(s) = loc.address_2.as_deref().filter(|s| !s.is_empty()) {
        parts.push(s);
    }
    let mut line = parts.join(", ");

    let city_state: String = match (loc.locality.as_deref(), loc.region.as_deref()) {
        (Some(c), Some(r)) if !c.is_empty() && !r.is_empty() => format!("{c}, {r}"),
        (Some(c), _) if !c.is_empty() => c.to_string(),
        (_, Some(r)) if !r.is_empty() => r.to_string(),
        _ => String::new(),
    };
    if !city_state.is_empty() {
        if !line.is_empty() {
            line.push_str(", ");
        }
        line.push_str(&city_state);
    }
    if let Some(zip) = loc.postal_code.as_deref().filter(|s| !s.is_empty()) {
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(zip);
    }
    line
}

/// Build a Google Maps search URL for "<name> <address>" (name + the
/// fully formatted street address). Falls back to lat/lng when neither
/// the name nor the address is available.
fn google_maps_url(name: Option<&str>, loc: &VenueLocation) -> Option<String> {
    let address = format_address(loc);
    let parts: Vec<&str> = [name, Some(address.as_str())]
        .into_iter()
        .flatten()
        .filter(|s| !s.is_empty())
        .collect();

    let query = if parts.is_empty() {
        format!("{},{}", loc.latitude?, loc.longitude?)
    } else {
        parts.join(" ")
    };

    let encoded: String = url::form_urlencoded::byte_serialize(query.as_bytes()).collect();
    Some(format!(
        "https://www.google.com/maps/search/?api=1&query={encoded}"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn loc() -> VenueLocation {
        VenueLocation {
            address_1: Some("2315 Broadway".to_string()),
            address_2: None,
            locality: Some("New York".to_string()),
            region: Some("NY".to_string()),
            postal_code: Some("10024".to_string()),
            country: None,
            neighborhood: Some("Upper West Side".to_string()),
            cross_street_1: None,
            cross_street_2: None,
            latitude: Some(40.787022),
            longitude: Some(-73.978129),
            url_slug: Some("new-york-ny".to_string()),
        }
    }

    #[test]
    fn formats_full_address() {
        assert_eq!(format_address(&loc()), "2315 Broadway, New York, NY 10024");
    }

    #[test]
    fn formats_address_with_address_2() {
        let mut l = loc();
        l.address_2 = Some("Suite 200".to_string());
        assert_eq!(
            format_address(&l),
            "2315 Broadway, Suite 200, New York, NY 10024"
        );
    }

    #[test]
    fn formats_address_falls_back_when_fields_missing() {
        let mut l = loc();
        l.address_1 = None;
        l.postal_code = None;
        assert_eq!(format_address(&l), "New York, NY");
    }

    #[test]
    fn google_maps_url_uses_name_and_address() {
        assert_eq!(
            google_maps_url(Some("Maison Pickle"), &loc()).as_deref(),
            Some(
                "https://www.google.com/maps/search/?api=1&query=Maison+Pickle+2315+Broadway%2C+New+York%2C+NY+10024"
            ),
        );
    }

    #[test]
    fn google_maps_url_falls_back_to_lat_lng() {
        let mut l = loc();
        l.address_1 = None;
        l.locality = None;
        l.region = None;
        l.postal_code = None;
        assert_eq!(
            google_maps_url(None, &l).as_deref(),
            Some("https://www.google.com/maps/search/?api=1&query=40.787022%2C-73.978129"),
        );
    }

    #[test]
    fn google_maps_url_returns_none_with_no_inputs() {
        let mut l = loc();
        l.address_1 = None;
        l.locality = None;
        l.region = None;
        l.postal_code = None;
        l.longitude = None;
        assert_eq!(google_maps_url(None, &l), None);
    }
}
