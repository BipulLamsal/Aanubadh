## Development on progress
This is a translation tool developed for KU research lab for tmt project. I found there is significant gap in the pdf extractor tool that preserves layout maybe we can forward this idea and furhter work on it helping the os community and improve research activity.   

## Where our previous idea failed
    - At first we started with doc-rs and picking mut ref to runnable elements inside the paragraph and translating one to one and mutating the orignal value to translated one. It was working for simpler document but turns out aspects like (images inside tabel) unimplemeneted from the crate itself.  
    - we quickly moved to working with w-t (parsing xml direclty with quick-xml) grabing only the text and storing it on the vec and reconstrcting with same element we received form the parser. (quick and easy solution)
    - to limit the async task we have used sempaphore right now its 50, but I was planning it to store in state where user can set their need as they need 
    - another trick which we tried to implement failed is batching, well translation was smart enoguht to remove any type of unicodes and symbols. Even then we tried to use special token : `Aldrep` maping with nepali and tamang result but this failed because it ruins the context of the setence and puts comma everytime there is fullstops in the sentence.
    - Another trick was wasm but this was not possible because we were calling api inside so we switched to good old day tcp.
    - PDF is another problem but we are using very maintained library/tooling : pdf2htmlEX, converting pdf to a very high quality html and you know borwser lets you open html as pdf so why worry let browser worry.   
    - PDF was the toughest part of the whole project

## Architecutre Right Now:
I am just bored I dont wanna write.


