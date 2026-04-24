"Simple" iced based program to rappidly display a directory of images.

# History
"Eons" ago back in the pre-windows days of MS-DOS I liked to be able to _quickly_ view a directory of images full screen. I don't actually remember which program I used back then, but eventually when Windows 95 became "a thing" and then somewhat after that even, I started using ACDSee.  At some point so many "features" got added on to the program that it stopped being FAST anymore for MY inteneded use.  I also was using Linux more and wanted something that worked.  I discovered feh.  Feh was aweful, but actually accomplised what I wanted.  I used that for while, but eventually gave up on it and started developing a PyQt app to replace it for MY use.  In the "fullness of time" that program has become "bloated" with stuff that I use EVERY day, but makes it unsuitable for anothing other than MY personal use. In early April of '26, I was experimenting with Rust and Iced as a UI framework.  This version of Quick Viewer was born.

# Intention
My intention for this version of the application is to keep it as a foundation for "quickly viewing a directory of images."  I don't want to add too many features beyond that.  This code is likely to become "foundational" in that I'd like to eventually transform it into a crate that I can build on.  As of TODAY (4/24/26) I don't know when that will happen.


# Comments
Just somethings I've been musing on...

I don't like that Iced doesn't support DPI Aware (on Windows) _directly_.  
This is only a mild annoyance, but does impact my usage of Iced a little when handling very large images.

I am uncertain if Iced will continue to be a foundational UI codebase for any future Rust development. I am concered about future breaking changes that COULD be detrimental to what I wish to do. I generally LIKE how Iced it setup and the foundation Iced is built on is nice.  The amount of "boilerplate" code needed makes sustainabilty potentially a bad thing.  Don't know if anyone has done this, but something "Like" a series of proc_macros to do the basics of "stuff" would be handy.  My thoughts are along the lines of:

This is just the mockup of what I've been considering.  For both sippers and "sub widgets" it would greately increase the ease of implementation thinking about the "download_progress" example.
```rust
enum Message{
    Action1,
    sipper_message!(Gloink),
}

fn update ... {
    sipper_handler!(Gloink)
}

sipper_body!(Gloink) 
```



