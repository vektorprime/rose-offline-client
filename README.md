# rose-offline-client
The original version is made by exjam, all credits to him.
An open source client for ROSE Online, compatible with the official 129_129en irose server or [rose-offline](https://github.com/exjam/rose-offline/).


This is a fork modified with an LLM. I take no credit for this and can't speak to the code structure as it's evolved past my basic understanding of Bevy.



## Features added by AI
-Bevy has been upgraded to 0.16.1.
-expanded settings for most of the new features
leaves and grass swaying in the wind
-volumetric fog (disabled by default)
-depth of field (disabled by default)
-SSAO
-better shadows
-water quality changes
-animated fish in water
-animated birds in sky
-seasons/weather (leaves falling, snow, rain, thunder and lightning)
-character dashes (dirt kicked up or cloud of dust when running) when running


# Screenshots

## Leaves and grass swaying in wind

https://github.com/user-attachments/assets/ba460143-2e7d-40ae-899e-825f8deba277

## Bird flying around

<img width="1775" height="998" alt="rose-birds" src="https://github.com/user-attachments/assets/dea30a9d-a154-4ded-9d80-804c89d7cf26" />

## Depth of Field
Depth of field OFF
<img width="1421" height="917" alt="rose-depth-of-field-off" src="https://github.com/user-attachments/assets/6f9e0c04-f627-4303-9ee9-ccd5fc7e34eb" />
Depth of Field On
<img width="1564" height="899" alt="rose-depth-of-field-on" src="https://github.com/user-attachments/assets/4daadab3-ac82-40a1-8016-68924337d9c5" />

## Dirt Dash while running
<img width="1337" height="901" alt="rose-dirt-dash" src="https://github.com/user-attachments/assets/723eaec5-0abf-46e8-a92e-36f4a70ffd13" />

## Seasons
These actually turned out REALLY good
### Fall
<img width="1582" height="1007" alt="rose-season-fall" src="https://github.com/user-attachments/assets/cdfa7551-e907-4a46-ac35-5ae312657f5e" />

### Spring
<img width="1468" height="1055" alt="rose-season-spring" src="https://github.com/user-attachments/assets/55025118-8864-4927-a0bb-daa96d1d661b" />

### Winter
<img width="1650" height="993" alt="rose-season-winter" src="https://github.com/user-attachments/assets/92a225f8-2353-40b2-bc71-812067a0cc0d" />

### Summer (still a work in progress, not very good)
Grass is meant to grow everywhere, but it doesn't look very good now. I will probably ask the LLM to modify this so the grass models are multiplied instead of generating the grass.

# Running
Run rose-offline-client from your installed official client directory (the folder containing data.idx), or you can use the `--data-idx` or `--data-path` arguments as described below.


## Optional arguments:
- `--data-idx=<path/to/data.idx>` Path to irose 129en data.idx
- `--data-aruavfs-idx=<path/to/data.idx>` Path to aruarose data.idx
- `--data-titanvfs-idx=<path/to/data.idx>` Path to titanrose data.idx
- `--ip` Server IP for login server (defaults to 127.0.0.1)
- `--port` Server port for login server (defaults to 29000)
- `--model-viewer` Start the client in model viewer mode
- `--zone=<N>` Start the client in zone viewer mode in the given zone

## Auto login arguments:
- `--auto-login` Automatic login.
- `--username=<username>` Username for auto login
- `--password=<password>` Password for auto login
- `--server-id=<N>` Server ID for auto login (defaults to 0)
- `--channel-id=<N>` Channel ID for auto login (defaults to 0)
- `--character-name=<name>` Character name for auto login (optional, auto login can be username/password only)

# Screenshots

<img alt="Fighting Jellybeans"  src="https://user-images.githubusercontent.com/1302758/218569716-d7c131e0-bc5b-4474-b060-745755202c95.jpg">

<img alt="Castlegear" src="https://user-images.githubusercontent.com/1302758/218569729-11887740-2205-4730-a420-c21b2e8a83f2.jpg">

