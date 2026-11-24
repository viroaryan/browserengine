//! Built-in sovereign ruleset covering high-impact tracking and ad networks.

pub const BUILTIN_RULESET: &str = r#"
! ----------------------------------------------------
! Nirvana Sovereign AdBlock & Privacy Protection Rules
! ----------------------------------------------------

! Major Ad Networks & Trackers
||doubleclick.net^
||googlesyndication.com^
||googleadservices.com^
||adservice.google.com^
||pagead2.googlesyndication.com^
||google-analytics.com^
||googletagmanager.com^
||analytics.google.com^
||pixel.facebook.com^
||connect.facebook.net/en_US/fbevents.js^
||criteo.com^
||criteo.net^
||taboola.com^
||outbrain.com^
||adnxs.com^
||rubiconproject.com^
||pubmatic.com^
||amazon-adsystem.com^
||scorecardresearch.com^
||hotjar.com^
||quantserve.com^
||adcolony.com^
||unityads.unity3d.com^
||vungle.com^
||chartboost.com^

! Common Ad Paths
*/ads/*$image,script
*/adserver/*$image,script
*/advertisement/*$image,script
*/banner/ad_*$image
*/popups/ad_*$script

! Global Cosmetic Element Hiding Filters
##.adsbygoogle
##.ad-banner
##.advertisement
##.sponsored-post
##.sponsor-banner
###ad-container
###ad-wrapper
###ad-banner
###google_ads_iframe
##ins.adsbygoogle
##div[class*="sponsored"]
##div[id*="google_ads"]
"#;
