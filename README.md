# Yafpm
Yafpm (Yet Another Functional Package Manager – likely not the final name)
is a system package manager largely inspired by the
[Nix Package Manager](https://nixos.org/) and released under the terms of
the GNU General Public License version 2 or later.

## Design
Yafpm takes its primary design inspiration from the Nix package manager.
However, it is not a rewrite of Nix, but is exploring other decisions in the
design space around Nix. An overview of some of these decisions is found in
[A Critique of Nix Package Manager](https://www.iohannes.us/en/commentary/nix-critique/).

## Usage
Since this project is still in its early days, there is only one command:
`yafpm-build`. This command will build and install a package that is
described by a TOML or JSON file. An example of this file is given in the
`examples` directory. Currently there is no way to run a repository, or
install a package from a repository, or uninstall anything (aside from
using `rm`).

## License
Yafpm is offered under the terms of the GNU General Public License
version 2 or later. This is found in the file named `LICENSE`.

## Contributing
Contributions are welcome! Please send your patch by Github pull request.
Contributions must be given with a real name and email address (no pseudonyms
or `@noreply.github.com` addresses, sorry), and with a commit message that ends
`Signed-off-by: [name] [email]`, to show that you have read and are meeting the
terms of the Developer Certificate of Origin version 1.1, which is found in the
file `DeveloperCertificate.txt`.
