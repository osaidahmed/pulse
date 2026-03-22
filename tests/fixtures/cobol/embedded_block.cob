       IDENTIFICATION DIVISION.
       PROGRAM-ID. EMBED-PROG.
       DATA DIVISION.
       WORKING-STORAGE SECTION.
       01 WS-MSG PIC X(500).
       PROCEDURE DIVISION.
       LARGE-BLOCK.
           MOVE "Line 1 of a very long message that spans
      -    "many continuation lines in the source code
      -    "and keeps going with more text content here
      -    "and even more text to fill the block up to
      -    "the threshold that triggers the smell for a
      -    "large embedded block detection in the code
      -    "analysis tool that checks for oversized text
      -    "blocks within function bodies so we add some
      -    "more lines to ensure we cross the threshold
      -    "for the large embedded block smell detector
      -    "which requires at least sixteen lines of text
      -    "in a single string literal or block constant
      -    "and here we go with line thirteen of content
      -    "followed by line fourteen right after that
      -    "then line fifteen coming up very shortly now
      -    "and finally line sixteen to cross threshold
      -    "with line seventeen for good measure here"
               TO WS-MSG.
           DISPLAY WS-MSG.
       SMALL-BLOCK.
           MOVE "Short message" TO WS-MSG.
           DISPLAY WS-MSG.
