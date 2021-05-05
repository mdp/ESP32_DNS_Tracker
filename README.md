# ESP32 DNS Location Tracker

Goal: To track a esp32 device without a cellular connection or GPS receiver, using "open" wifi access points, DNS tunnelling and geocoding scanned WiFi MAC addresses.

## How it works

The ESP32 device periodically scans it's WiFi environment and gets a list of the 10 strongest AP's and the MAC addressed. If one of those AP's is "open" (but can be a captive portal or paid wifi service), it connects to the AP, then encodes the list of 10 AP MAC addresses into a DNS query to a server we control. When we see that query, we decode it, get a list of the MAC addresses and use Google's Geocoding API to return an approximate location. Because of the ESP32's ability to deep-sleep, we can power a unit for several months off one battery.

### Why DNS tunneling works

DNS tunneling works because even in the case of a restricted internet portal, the AP still needs to give back an honest response to a DNS query. For example, if you lookup www.yahoo.com, it will resolve it correctly to the records IP and then when you attempt to connect to that IP, redirect you to the captive portal. Instead of letting you simple make your own DNS query, the AP will typically proxy the request via it's own DNS resolution service. Because a DNS record can be up to 253 characters long you can actually fit quite a bit of data into a single query encoded as Base32. In fact, it's enough to encode 10 MAC addresses along with their channels and signal strength.

The format we use is as follows:

[Version 1][Index 1][UniqueID 13][Message ...].domainwecontrol.com

The index quint (A single base32 character) allow for up to 32 messages to be split apart and reasssembled on the server side and preserves ordering.

## The Code

This project breaks it's code up into two parts, the Arduino code for the ESP32 and a DNS server writter in Rust. It's very experimental and not designed for any serious use. It should be viewed more as a proof of concept.

## See it in action

Coming soon....

