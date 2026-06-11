       IDENTIFICATION DIVISION.
       PROGRAM-ID. SHAPES-PROG.
       DATA DIVISION.
       WORKING-STORAGE SECTION.
       01 WS-A PIC 9(3).
       01 WS-B PIC 9(3).
       01 WS-C PIC 9(3).
       01 WS-D PIC 9(3).
       01 WS-X PIC 9(3).
       01 WS-IDX PIC 9(3).
       01 WS-TOTAL PIC 9(5).
       01 WS-RESULT PIC 9(3).
       PROCEDURE DIVISION.
       FLAT-CALLS.
           MOVE 1 TO WS-A.
           MOVE 2 TO WS-B.
           ADD WS-A TO WS-B GIVING WS-C.
           MOVE WS-C TO WS-RESULT.
       PICK-BRANCH.
           IF WS-X > 10
               MOVE 3 TO WS-RESULT
           ELSE
               IF WS-X > 5
                   MOVE 2 TO WS-RESULT
               ELSE
                   MOVE 1 TO WS-RESULT
               END-IF
           END-IF.
       NESTED-GUARD.
           IF WS-A > 0
               IF WS-B > 0
                   IF WS-C > 0
                       MOVE 1 TO WS-RESULT
                   END-IF
               END-IF
           END-IF.
       LOOP-FILTER.
           MOVE 0 TO WS-TOTAL.
           MOVE 0 TO WS-IDX.
           PERFORM UNTIL WS-IDX > 10
               IF WS-A > 0
                   ADD WS-A TO WS-TOTAL
               END-IF
               ADD 1 TO WS-IDX
           END-PERFORM.
       BOOL-BLEND.
           IF (WS-A > 0 AND WS-B > 0) OR (WS-C > 0 AND WS-D > 0)
               MOVE 1 TO WS-RESULT
           END-IF.
