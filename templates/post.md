---
title: "{{ form.title }}"
date: "{{ form.date }}"
categories: "{{ form.categories }}"
main:
  image: "{{ form.main.image_url }}"
  alt_text: "{{ form.main.alt_text }}"
  caption: "{{ form.main.caption }}"
  location: "{{ form.main.location }}"
  coordinates: "{{ form.main.coordinates }}"
  coordinates_url: "https://www.google.com/maps/place/{{ form.main.coordinates }}"
{% if form.strava %}strava: "{{ form.strava }}"{% endif %}
---

{{ form.main.description }}

{% for key, metadata in form.images -%}
{%- if key == form.main.file_name %}{% continue %}{% endif -%}
![{{ metadata.alt_text }}]({{ metadata.image_url }})
{%- if metadata.location %}
*[{{ metadata.location }}](https://www.google.com/maps/place/{{ metadata.coordinates }}): {{ metadata.caption }}*
{% endif %}
{%- if metadata.description %}
{{ metadata.description }}
{% endif %}
