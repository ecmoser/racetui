# Overall Vision:

Implement live scoring tracking, standings for finished races, drivers and constructors seasonal standings (where applicable), and many more features 

## Live data

Implement live track maps/leaderboards/timing pages for any series that provide them via free APIs or website scraping (see https://github.com/dk-a-dev/termf1 for an example for f1). 

Ex: F1 provides live track maps: implement that in the tui so that it shows drivers circulating around the track.

Implement live timing tables for all series where that is fetchable.

Add a page for series championship standings, (ex. for f1 both driver and constructor)

Add results for finished races, including points earned and other useful information that can be viewed by selecting a finished race. Include any tire information or other stuff that can be found online.

Send configurable system notifications when events are about to start (customize to be all series, only favorites, only certain series, only certain sesssions, etc.)


# How to write a plan

Assume it will be implemented by a weaker AI than yourself, so make it very descriptive and easy to follow

Break it into single commit-sized steps where the implementer will pause for human approval

Ask me about any design or implementation decisions (don't make any on your own).

Figure out all desing and implementation stuff now, don't leave anything undecided.
