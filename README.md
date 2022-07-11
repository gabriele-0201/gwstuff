# GWSTUFF

...

## RoadMap

MVP: 

+ [x] Render on focused screen
+ [x] layer alignment
+ [x] text from line arguments
+ [x] Line text alignment
+ [x] General style
    + [x] Config file parser
        + [x] bg_color
        + [x] bg_trasparency
        + [x] padding
        + [x] margins
        + [x] font_name
        + [x] font_size
        + [x] font_color
        + [x] intra_line_space
+ [x] Font Proper Scaling
+ [x] Timer - SORT OF - not really beautifull

Future: 
+ [ ] layer render only on specified screen


<div id="top"></div>

[![Contributors][contributors-shield]][contributors-url]
[![Forks][forks-shield]][forks-url]
[![Stargazers][stars-shield]][stars-url]
[![Issues][issues-shield]][issues-url]
[![MIT License][license-shield]][license-url]


<!-- PROJECT LOGO -->
<br />
<div align="center">

  <h3 align="center">Welcome to gwstuff!</h3>

  <p align="center">
    <a href="https://github.com/gabriele-0201/gwstuff/"><strong>Explore the docs »</strong></a>
    <br />
    <br />
    <a href="https://github.com/gabriele-0201/gwstuff/">View Demo</a>
    ·
    <a href="https://github.com/gabriele-0201/gwstuff/issues">Report Bug</a>
    ·
    <a href="https://github.com/gabriele-0201/gwstuff/issues">Request Feature</a>
  </p>
</div>



<!-- TABLE OF CONTENTS -->
<details>
  <summary>Table of Contents</summary>
  <ol>
    <li>
      <a href="#about-the-project">About The Project</a>
      <ul>
        <li><a href="#built-with">Built With</a></li>
      </ul>
    </li>
    <li>
      <a href="#getting-started">Getting Started</a>
      <ul>
        <li><a href="#prerequisites">Prerequisites</a></li>
        <li><a href="#installation">Installation</a></li>
      </ul>
    </li>
    <li><a href="#usage">Usage</a></li>
    <li><a href="#roadmap">Roadmap</a></li>
    <li><a href="#contributing">Contributing</a></li>
    <li><a href="#license">License</a></li>
  </ol>
</details>



<!-- ABOUT THE PROJECT -->
## About The Project

<!-- [![Product Name Screen Shot][product-screenshot]] -->

Gwstuff is a deamon-less lightweight notification program for Wayland compositors that can show a box with customizable text in it.




### Built With

* [Smithay client toolkit](https://github.com/Smithay/client-toolkit)



### Prerequisites

In order to use gwstuff you should be running a **Wayland compositor** (like Sway, GNOME, KDE, ...).
You will also need [fontconfig](https://www.freedesktop.org/wiki/Software/fontconfig/)
To compile gwstuff from source you should have **rustc** and **cargo** installed



### Installation

1. Clone the repo
   ```sh
   git clone https://github.com/gabriele-0201/gwstuff.git
   ```
2. Open it in a terminal
3. Run
```sh
cargo build --realease
```

Now the executable program should be at `./target/release/gwstuff` and you can run a test by executing
```sh
./target/release/gwstuff "hello world"
```

<p align="right">(<a href="#top">back to top</a>)</p>



<!-- USAGE EXAMPLES -->
## Usage

You can run the program with the default config just by using
```sh
gwstuff "your text here" "this text goes to a new line" "this text goes to the 3rd line"
```

Or you can specify a custom configuration following this steps:

- Create your own config file, according to the [template](https://github.com/gabriele-0201/gwstuff/example_config.toml)
- Note: the available fonts on your system can be listed with `fc-list`
- Run gwstuff specifying the **configuration name** (not filename):
  ```sh
  gwstuff --<myConfig> "your text here" "this text goes to the 2nd line" "this text goes to the 3rd line"
  ```

<!-- TODO: do a config format guide -->
<!--
- See the [config file format guide](https://github.com/gabriele-0201/gwstuff/blob/main/docs/config_format.md) to customize your gstuff
-->

Here are some example usecases:
- Getting important stats of your system
- Having a customizable slider for display backlight and/or volume
- Integrating fast notification in your own project
- Showing reminders or timers

<!-- TODO: add some examples in an example directory and add images here -->



<!-- ROADMAP -->
## TODO/Roadmap

See the [open issues](https://github.com/gabriele-0201/gwstuff/issues) for a list of proposed features (and known issues).



<!-- CONTRIBUTING -->
## Contributing

If you have a suggestion that would make the project better, fork the repo and create a pull request. You can also simply open an issue with the tag "enhancement".
Don't forget to give the project a star if you liked it!

<p align="right">(<a href="#top">back to top</a>)</p>



<!-- LICENSE -->
## License

Distributed under the MIT License. See `LICENSE.txt` for more information.



<!-- MARKDOWN LINKS & IMAGES -->
<!-- https://www.markdownguide.org/basic-syntax/#reference-style-links -->
[contributors-shield]: https://img.shields.io/github/contributors/gabriele-0201/gwstuff.svg?style=for-the-badge
[forks-shield]: https://img.shields.io/github/forks/gabriele-0201/gwstuff.svg?style=for-the-badge
[stars-shield]: https://img.shields.io/github/stars/gabriele-0201/gwstuff.svg?style=for-the-badge
[issues-shield]: https://img.shields.io/github/issues/gabriele-0201/gwstuff.svg?style=for-the-badge
[license-shield]: https://img.shields.io/github/license/gabriele-0201/gwstuff.svg?style=for-the-badge
[license-url]: https://github.com/gabriele-0201/gwstuff/blob/master/LICENSE.txt
[contributors-url]: https://github.com/gabriele-0201/gwstuff/contributors
[forks-url]: https://github.com/gabriele-0201/gwstuff/network/members
[stars-url]: https://github.com/gabriele-0201/gwstuff/stargazers
[issues-url]: https://github.com/gabriele-0201/gwstuff/issues

