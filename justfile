id := "st.lynx.plugins.opendeck-akp03.sdPlugin"

release next=`git cliff --bumped-version | tr -d "v"`: (bump next) package (tag next)

package: build-linux build-mac build-win collect zip

bump next=`git cliff --bumped-version | tr -d "v"`:
    git diff --cached --exit-code

    echo "We will bump version to {{next}}, press any key"
    read ans

    sed -i 's/"Version": ".*"/"Version": "{{next}}"/g' manifest.json
    sed -i 's/^version = ".*"$/version = "{{next}}"/g' Cargo.toml

tag next=`git cliff --bumped-version | tr -d "v"`:
    echo "Generating changelog"
    git cliff -o CHANGELOG.md --tag v{{next}}

    echo "We will now commit the changes, please review before pressing any key"
    read ans

    git add .
    git commit -m "chore(release): v{{next}}"
    git tag "v{{next}}"

build-linux:
    cargo build --release --target x86_64-unknown-linux-gnu --target-dir target/plugin-linux

build-mac:
    docker run --rm -it -v $(pwd):/io -w /io ghcr.io/rust-cross/cargo-zigbuild:sha-eba2d7e cargo zigbuild --release --target universal2-apple-darwin --target-dir target/plugin-mac

build-win:
    cargo build --release --target x86_64-pc-windows-gnu --target-dir target/plugin-win

clean:
    sudo rm -rf target/

collect:
    rm -rf build
    mkdir -p build/{{id}}
    cp -r assets build/{{id}}
    cp manifest.json build/{{id}}
    cp target/plugin-linux/x86_64-unknown-linux-gnu/release/opendeck-akp03 build/{{id}}/opendeck-akp03-linux
    cp target/plugin-mac/universal2-apple-darwin/release/opendeck-akp03 build/{{id}}/opendeck-akp03-mac
    cp target/plugin-win/x86_64-pc-windows-gnu/release/opendeck-akp03.exe build/{{id}}/opendeck-akp03-win.exe

[working-directory: "build"]
zip:
    zip -r opendeck-akp03.plugin.zip {{id}}/
