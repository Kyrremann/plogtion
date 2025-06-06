---
title: "{{ form.title }}"
date: {{ form.date }}
categories: {{ form.categories }}
image: {{ form.main_image }}
---

{% for key, metadata in form.images -%}
{{ metadata.description }}
{%- if metadata.image == form.main_image -%}
{%- else -%}
![{{ metadata.alt_text }}]({{ metadata.image }})
{{ metadata.description }}
{{ metadata.location }}
{% endif -%}
{% endfor -%}
