import sys, argparse, json, urllib.request

GOOGLE_API_URL="https://www.googleapis.com/geolocation/v1/geolocate"

def bytes_from_file(filename, chunksize=8192):
    byte_arr = []
    with open(filename, "rb") as f:
        while True:
            chunk = f.read(chunksize)
            if chunk:
                for b in chunk:
                    byte_arr.append(b)
            else:
                break
    return bytearray(byte_arr)

def to_mac(mac_bytes):
    return ':'.join('%02x' % b for b in mac_bytes)

def build_json(input_file):
    byte_arr = bytes_from_file(input_file)
    geo = {
      "considerIp": False,
      "wifiAccessPoints": []
    }
    wifi_n = byte_arr[1]
    print(f"Found {wifi_n} networks.")
    offset = 2
    for n in range(0, wifi_n):
        start = n * 8 + offset
        mac = to_mac(byte_arr[start:start+6])
        channel = byte_arr[start+6]
        rssi = int.from_bytes([byte_arr[start+7]], byteorder='big', signed=True)
        geo["wifiAccessPoints"].append({
            "macAddress": mac,
            "signalStrength": rssi,
            "channel": channel
        })
    wifi_name = str(byte_arr[wifi_n * 8 + offset:], 'UTF-8')
    print(f"Connected to '{wifi_name}'")
    json_str = json.dumps(geo)
    print(json_str)
    return json_str

def geocode(json_str, api_key):
    req = urllib.request.Request(f"{GOOGLE_API_URL}?key={api_key}", data=json_str.encode('utf8'),
        headers={'content-type': 'application/json'})
    response = urllib.request.urlopen(req)
    response_str = response.read().decode('utf8')
    response_json = json.loads(response_str)
    print(response_str)
    print(f"https://www.google.com/maps/search/?api=1&query={response_json['location']['lat']},{response_json['location']['lng']}")



def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("input_file")
    parser.add_argument("--geokey", help="Your Google API key for geocoding")
    args = parser.parse_args()
    json_str = build_json(args.input_file)
    if args.geokey:
        print("Geocoding...")
        geocode(json_str, args.geokey)

if __name__ == "__main__":
   main()
