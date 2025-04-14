X=(peggen-core peggen-macs peggen)
for x in ${X[@]}
do
    echo $x
    cargo publish -p $x
done