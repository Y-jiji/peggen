X=(peggen-core peggen-macs peggen peggen-examples)
for x in $X
do
    echo $x
    cargo publish -p $x
done