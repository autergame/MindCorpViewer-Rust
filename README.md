# MindCorpViewer-Rust
League Of Legends Model Viewer Using Rust

## Help From:
* https://github.com/Querijn/LeagueModel
* https://github.com/CommunityDragon/CDTB
* https://github.com/LoL-Fantome/LeagueToolkit

## How to use: 
* Step 1: Extract skn, skl, dds files and animations folder with: https://github.com/Crauzer/Obsidian
* Step 2: Modify the config.json
```json
{
	"Vsync": false,
	"ShowFloor": true,
	"ShowSkybox": true,
	"SynchronizedTime": false,
	"PATHS": [
	  {
		"Name": "Template", <-- Optional Name
		"DDS": "template", <-- Path to dds folder
		"SKN": "template/template.skn", <-- SKN file
		"SKL": "template/template.skl", <-- SKL file
		"Animations": "template/animations" <-- Path to animations folder
	  }
	],
	"OPTIONS": [ <-- Do not remove
	],
	"MESHES": [ <-- Do not remove
	]
}
```


![alt text](MindCorpViewer-Rust_image.png)