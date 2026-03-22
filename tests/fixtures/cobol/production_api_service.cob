      *> API-style COBOL service with EVALUATE-heavy logic.
       IDENTIFICATION DIVISION.
       PROGRAM-ID. API-SERVICE.
       DATA DIVISION.
       WORKING-STORAGE SECTION.
       01 WS-REQUEST-TYPE PIC X(10).
       01 WS-AUTH-TOKEN PIC X(40).
       01 WS-USER-ROLE PIC X(10).
       01 WS-RESOURCE PIC X(20).
       01 WS-RESPONSE PIC X(80).
       01 WS-STATUS-CODE PIC 9(3).
       01 WS-VALID PIC 9.
       01 WS-RETRIES PIC 9.
       01 WS-MAX-RETRY PIC 9.
       01 WS-LOG-MSG PIC X(80).
       PROCEDURE DIVISION.
       HANDLE-REQUEST.
           EVALUATE WS-REQUEST-TYPE
               WHEN "GET"
                   PERFORM HANDLE-GET
               WHEN "POST"
                   PERFORM HANDLE-POST
               WHEN "PUT"
                   PERFORM HANDLE-PUT
               WHEN "DELETE"
                   PERFORM HANDLE-DELETE
               WHEN OTHER
                   MOVE 405 TO WS-STATUS-CODE
                   MOVE "Method Not Allowed" TO WS-RESPONSE
           END-EVALUATE.
           IF WS-STATUS-CODE > 399
               PERFORM LOG-ERROR
           END-IF.
       HANDLE-GET.
           IF WS-AUTH-TOKEN = SPACES
               MOVE 401 TO WS-STATUS-CODE
           ELSE
               EVALUATE WS-USER-ROLE
                   WHEN "ADMIN"
                       MOVE 200 TO WS-STATUS-CODE
                       MOVE "Full access" TO WS-RESPONSE
                   WHEN "USER"
                       IF WS-RESOURCE = "PRIVATE"
                           MOVE 403 TO WS-STATUS-CODE
                       ELSE
                           MOVE 200 TO WS-STATUS-CODE
                       END-IF
                   WHEN OTHER
                       MOVE 403 TO WS-STATUS-CODE
               END-EVALUATE
           END-IF.
       HANDLE-POST.
           IF WS-VALID = 1
               MOVE 201 TO WS-STATUS-CODE
           ELSE
               MOVE 400 TO WS-STATUS-CODE
           END-IF.
       HANDLE-PUT.
           IF WS-VALID = 1
               MOVE 200 TO WS-STATUS-CODE
           ELSE
               MOVE 400 TO WS-STATUS-CODE
           END-IF.
       HANDLE-DELETE.
           IF WS-AUTH-TOKEN = SPACES
               MOVE 401 TO WS-STATUS-CODE
           ELSE
               MOVE 204 TO WS-STATUS-CODE
           END-IF.
       VALIDATE-INPUT.
           IF WS-REQUEST-TYPE = SPACES
               MOVE 0 TO WS-VALID
           ELSE
               IF WS-AUTH-TOKEN = SPACES
                   IF WS-REQUEST-TYPE NOT = "GET"
                       MOVE 0 TO WS-VALID
                   ELSE
                       MOVE 1 TO WS-VALID
                   END-IF
               ELSE
                   MOVE 1 TO WS-VALID
               END-IF
           END-IF.
       LOG-ERROR.
           STRING "Error: " DELIMITED SIZE
                  WS-STATUS-CODE DELIMITED SIZE
                  " " DELIMITED SIZE
                  WS-RESPONSE DELIMITED SIZE
                  INTO WS-LOG-MSG
           END-STRING.
           DISPLAY WS-LOG-MSG.
       LOG-INFO.
           STRING "Info: " DELIMITED SIZE
                  WS-STATUS-CODE DELIMITED SIZE
                  " " DELIMITED SIZE
                  WS-RESPONSE DELIMITED SIZE
                  INTO WS-LOG-MSG
           END-STRING.
           DISPLAY WS-LOG-MSG.
