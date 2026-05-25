### Non-Real Time (7EH)

| SUB-ID #1 | SUB-ID #2 | DESCRIPTION |
| :--- | :--- | :--- |
| 00 | -- | Unused  |
| 01 | -- | Sample Dump Header  |
| 02 | -- | Sample Data Packet  |
| 03 | -- | Sample Dump Request  |
| 04 | -- | MIDI Time Code  |
| | 00 | Special  |
| | 01 | Punch In Points  |
| | 02 | Punch Out Points  |
| | 03 | Delete Punch In Point  |
| | 04 | Delete Punch Out Point  |
| | 05 | Event Start Point  |
| | 06 | Event Stop Point  |
| | 07 | Event Start Points with additional info.  |
| | 08 | Event Stop Points with additional info.  |
| | 09 | Delete Event Start Point  |
| | 0A | Delete Event Stop Point  |
| | 0B | Cue Points  |
| | 0C | Cue Points with additional info.  |
| | 0D | Delete Cue Point  |
| | 0E | Event Name in additional info.  |
| 05 | nn | Sample Dump Extensions  |
| | 01 | Loop Points Transmission  |
| | 02 | Loop Points Request  |
| | 03 | Sample Name Transmission  |
| | 04 | Sample Name Request  |
| | 05 | Extended Dump Header  |
| | 06 | Extended Loop Points Transmission  |
| | 07 | Extended Loop Points Request  |
| 06 | no | General Information  |
| | 01 | Identity Request  |
| | 02 | Identity Reply  |
| 07 | -- | File Dump  |
| | 01 | Header  |
| | 02 | Data Packet  |
| | 03 | Request  |
| 08 | -- | MIDI Tuning Standard (Non-Real Time)  |
| | 00 | Bulk Dump Request  |
| | 01 | Bulk Dump Reply  |
| | 03 | Tuning Dump Request  |
| | 04 | Key-Based Tuning Dump  |
| | 05 | Scale/Octave Tuning Dump, 1 byte format  |
| | 06 | Scale/Octave Tuning Dump, 2 byte format  |
| | 07 | Single Note Tuning Change with Bank Select  |
| | 08 | Scale/Octave Tuning, 1 byte format  |
| | 09 | Scale/Octave Tuning, 2 byte format  |
| 09 | nn | General MIDI  |
| | 01 | General MIDI 1  |
| | 02 | General MIDI  |
| | 03 | General MIDI 2  |
| 0A | nn | Downloadable Sounds  |
| | 01 | Turn DLS On  |
| | 02 | Turn DLS Off  |
| | 03 | Turn DLS Voice Allocation Off  |
| | 04 | Turn DLS Voice Allocation On  |
| 0B | nn | File Reference Message  |
| | 00 | reserved (do not use)  |
| | 01 | Open File  |
| | 02 | Select or Reselect Contents |
| | 03 | Open File and Select Contents |
| | 04 | Close File |
| | 05-7F | reserved (do not use) |
| 0C | nn | MIDI Visual Control |
| | 00-7F | MVC Commands (See MVC Documentation) |
| 0D | nn | MIDI Capability Inquiry |
| | 00-7F | Inquiry/Response Messages (See Documentation) |
| 7B | -- | End of File |
| 7C | -- | Wait |
| 7D | -- | Cancel |
| 7E | -- | NAK |
| 7F | -- | ACK |

### Real Time (7FH)

| SUB-ID #1 | SUB-ID #2 | DESCRIPTION |
| :--- | :--- | :--- |
| 00 | -- | Unused |
| 01 | nn | MIDI Time Code |
| | 01 | Full Message |
| | 02 | User Bits |
| 02 | nn | MIDI Show Control |
| | 00 | MSC Extensions |
| | 01-7F | MSC Commands (see MSC Documentation) |
| 03 | nn | Notation Information |
| | 01 | Bar Number |
| | 02 | Time Signature (Immediate) |
| | 42 | Time Signature (Delayed) |
| 04 | nn | Device Control |
| | 01 | Master Volume |
| | 02 | Master Balance |
| | 03 | Master Fine Tuning |
| | 04 | Master Coarse Tuning |
| | 05 | Global Parameter Control |
| 05 | no | Real Time MTC Cueing |
| | 00 | Special |
| | 01 | Punch In Points |
| | 02 | Punch Out Points |
| | 03 | (Reserved) |
| | 04 | (Reserved) |
| | 05 | Event Start points |
| | 06 | Event Stop points |
| | 07 | Event Start points with additional info. |
| | 08 | Event Stop points with additional info. |
| | 09 | (Reserved) |
| | 0A | (Reserved) |
| | 0B | Cue points |
| | 0C | Cue points with additional info. |
| | 0D | (Reserved) |
| | 0E | Event Name in additional info. |
| 06 | -- | Machine Control Commands MIDI |
| | 00-7F | MMC Commands (See MMC Documentation) |
| 07 | -- | MIDI Machine Control Responses |
| | 00-7F | MMC Responses (See MMC Documentation) |
| 08 | nn | MIDI Tuning Standard (Real Time) |
| | 02 | Single Note Tuning Change |
| | 07 | Single Note Tuning Change with Bank Select |
| | 08 | Scale/Octave Tuning, 1 byte format |
| | 09 | Scale/Octave Tuning, 2 byte format |
| 09 | -- | Controller Destination Setting (See GM2 Documentation) |
| | 01 | Channel Pressure (Aftertouch) |
| | 02 | Polyphonic Key Pressure (Aftertouch) |
| | 03 | Controller (Control Change) |
| 0A | 01 | Key-based Instrument Control |
| 0B | 01 | Scalable Polyphony MIDI MIP Message |
| 0C | 00 | Mobile Phone Control Message |
